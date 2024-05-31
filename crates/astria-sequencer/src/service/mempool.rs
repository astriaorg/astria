use std::{
    collections::{
        HashSet,
        VecDeque,
    },
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
    },
};

use astria_core::{
    generated::protocol::transaction::v1alpha1 as raw,
    protocol::{
        abci::AbciErrorCode,
        transaction::v1alpha1::SignedTransaction,
    },
};
use cnidarium::Storage;
use futures::{
    Future,
    FutureExt,
};
use prost::Message as _;
use tendermint::v0_38::abci::{
    request,
    response,
    MempoolRequest,
    MempoolResponse,
};
use tokio::sync::RwLock;
use tower::Service;
use tower_abci::BoxError;
use tracing::Instrument as _;

use crate::{
    accounts::state_ext::StateReadExt,
    mempool::Mempool as AppMempool,
    metrics_init,
    transaction,
};

const MAX_TX_SIZE: usize = 256_000; // 256 KB
const CACHE_SIZE: usize = 4600;

#[derive(Clone)]
pub(crate) struct TxCache {
    map: HashSet<[u8; 32]>,
    list: VecDeque<[u8; 32]>,
    max_size: usize,
    size: usize,
}

impl TxCache {
    fn new(max_size: usize) -> Self {
        Self {
            map: HashSet::new(),
            list: VecDeque::with_capacity(max_size),
            max_size,
            size: 0,
        }
    }

    fn exists(&self, tx_hash: [u8; 32]) -> bool {
        self.map.contains(&tx_hash)
    }

    fn add(&mut self, tx_hash: [u8; 32]) {
        if self.map.contains(&tx_hash) {
            return;
        };
        if self.size == self.max_size {
            self.map.remove(
                &self
                    .list
                    .pop_front()
                    .expect("cache should contain elements"),
            );
        } else {
            self.size = self.size.checked_add(1).expect("cache size overflowed");
        }
        self.list.push_back(tx_hash);
        self.map.insert(tx_hash);
    }
}

/// Mempool handles [`request::CheckTx`] abci requests.
//
/// It performs a stateless check of the given transaction,
/// returning a [`tendermint::v0_38::abci::response::CheckTx`].
#[derive(Clone)]
pub(crate) struct Mempool {
    storage: Storage,
    mempool: AppMempool,
    tx_cache: Arc<RwLock<TxCache>>,
}

impl Mempool {
    pub(crate) fn new(storage: Storage, mempool: AppMempool) -> Self {
        Self {
            storage,
            mempool,
            tx_cache: Arc::new(RwLock::new(TxCache::new(CACHE_SIZE))),
        }
    }
}

impl Service<MempoolRequest> for Mempool {
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<MempoolResponse, BoxError>> + Send + 'static>>;
    type Response = MempoolResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: MempoolRequest) -> Self::Future {
        use penumbra_tower_trace::v038::RequestExt as _;
        let span = req.create_span();
        let storage = self.storage.clone();
        let mut mempool = self.mempool.clone();
        let mut tx_cache = self.tx_cache.clone();
        async move {
            let rsp = match req {
                MempoolRequest::CheckTx(req) => MempoolResponse::CheckTx(
                    handle_check_tx(req, storage.latest_snapshot(), &mut mempool, &mut tx_cache)
                        .await,
                ),
            };
            Ok(rsp)
        }
        .instrument(span)
        .boxed()
    }
}

/// Handles a [`request::CheckTx`] request.
///
/// Performs stateless checks (decoding and signature check),
/// as well as stateful checks (nonce and balance checks).
///
/// If the tx passes all checks, status code 0 is returned.
#[allow(clippy::too_many_lines)]
async fn handle_check_tx<S: StateReadExt + 'static>(
    req: request::CheckTx,
    state: S,
    mempool: &mut AppMempool,
    tx_cache: &mut Arc<RwLock<TxCache>>,
) -> response::CheckTx {
    use sha2::Digest as _;

    let tx_hash = sha2::Sha256::digest(&req.tx).into();

    if tx_cache.read().await.exists(tx_hash) {
        // transaction has already been seen, remove from cometbft mempool
        return response::CheckTx {
            code: AbciErrorCode::ALREADY_PROCESSED.into(),
            log: "transaction has already been processed by the mempool".to_string(),
            info: AbciErrorCode::TRANSACTION_TOO_LARGE.to_string(),
            ..response::CheckTx::default()
        };
    }

    // add to cache to avoid processing again
    tx_cache.write().await.add(tx_hash);

    let request::CheckTx {
        tx, ..
    } = req;
    if tx.len() > MAX_TX_SIZE {
        mempool.remove(tx_hash).await;
        metrics::counter!(metrics_init::CHECK_TX_REMOVED_TOO_LARGE).increment(1);
        return response::CheckTx {
            code: AbciErrorCode::TRANSACTION_TOO_LARGE.into(),
            log: format!(
                "transaction size too large; allowed: {MAX_TX_SIZE} bytes, got {}",
                tx.len()
            ),
            info: AbciErrorCode::TRANSACTION_TOO_LARGE.to_string(),
            ..response::CheckTx::default()
        };
    }

    let raw_signed_tx = match raw::SignedTransaction::decode(tx) {
        Ok(tx) => tx,
        Err(e) => {
            mempool.remove(tx_hash).await;
            return response::CheckTx {
                code: AbciErrorCode::INVALID_PARAMETER.into(),
                log: e.to_string(),
                info: "failed decoding bytes as a protobuf SignedTransaction".into(),
                ..response::CheckTx::default()
            };
        }
    };
    let signed_tx = match SignedTransaction::try_from_raw(raw_signed_tx) {
        Ok(tx) => tx,
        Err(e) => {
            mempool.remove(tx_hash).await;
            return response::CheckTx {
                code: AbciErrorCode::INVALID_PARAMETER.into(),
                info: "the provided bytes was not a valid protobuf-encoded SignedTransaction, or \
                       the signature was invalid"
                    .into(),
                log: e.to_string(),
                ..response::CheckTx::default()
            };
        }
    };

    if let Err(e) = transaction::check_stateless(&signed_tx).await {
        mempool.remove(tx_hash).await;
        metrics::counter!(metrics_init::CHECK_TX_REMOVED_FAILED_STATELESS).increment(1);
        return response::CheckTx {
            code: AbciErrorCode::INVALID_PARAMETER.into(),
            info: "transaction failed stateless check".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    if let Err(e) = transaction::check_nonce_mempool(&signed_tx, &state).await {
        mempool.remove(tx_hash).await;
        metrics::counter!(metrics_init::CHECK_TX_REMOVED_STALE_NONCE).increment(1);
        return response::CheckTx {
            code: AbciErrorCode::INVALID_NONCE.into(),
            info: "failed verifying transaction nonce".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    if let Err(e) = transaction::check_chain_id_mempool(&signed_tx, &state).await {
        mempool.remove(tx_hash).await;
        return response::CheckTx {
            code: AbciErrorCode::INVALID_CHAIN_ID.into(),
            info: "failed verifying chain id".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    }

    if let Err(e) = transaction::check_balance_mempool(&signed_tx, &state).await {
        mempool.remove(tx_hash).await;
        metrics::counter!(metrics_init::CHECK_TX_REMOVED_ACCOUNT_BALANCE).increment(1);
        return response::CheckTx {
            code: AbciErrorCode::INSUFFICIENT_FUNDS.into(),
            info: "failed verifying account balance".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    // tx is valid, push to mempool
    let current_account_nonce = state
        .get_account_nonce(*signed_tx.verification_key().address())
        .await
        .expect("can fetch account nonce");

    mempool
        .insert(signed_tx, current_account_nonce)
        .await
        .expect(
            "tx nonce is greater than or equal to current account nonce; this was checked in \
             check_nonce_mempool",
        );
    response::CheckTx::default()
}

mod test {
    #[tokio::test]
    async fn tx_cache_test() {
        use crate::service::mempool::TxCache;

        let mut tx_cache = TxCache::new(2);

        let tx_0 = [0u8; 32];
        let tx_1 = [1u8; 32];
        let tx_2 = [2u8; 32];

        assert!(
            !tx_cache.exists(tx_0),
            "no transaction should exist at first"
        );

        tx_cache.add(tx_0);
        assert!(tx_cache.exists(tx_0), "transaction was added, should exist");

        tx_cache.add(tx_1);
        tx_cache.add(tx_2);
        assert!(tx_cache.exists(tx_1), "transaction was added, should exist");
        assert!(tx_cache.exists(tx_2), "transaction was added, should exist");
        assert!(
            !tx_cache.exists(tx_0),
            "first transaction should be removed"
        );
    }
}
