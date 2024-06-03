use std::{
    collections::{
        HashMap,
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
use tendermint::{
    v0_38::abci::{
        request,
        response,
        MempoolRequest,
        MempoolResponse,
    },
    Time,
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

// TODO make these config configurable
const MAX_TX_SIZE: usize = 256_000; // 256 KB
const CACHE_SIZE: usize = 16384;
const CACHE_TTL: i64 = 60; // 60 seconds 

/// `TxCache` provides for keeping `CometBFT`'s mempool clean.
//
/// Since we're now using an app side mempool, we can remove
/// transactions from this mempool once we've placed them there.
///
/// The cache also places a TTL on the entries, allowing users
/// to request their transaction to be processed again after the
/// ttl has expired. This is useful for users who want to re-run a transaction
/// that failed to execute.
#[derive(Clone)]
pub(crate) struct TxCache {
    cache: HashSet<[u8; 32]>,
    time_added: HashMap<[u8; 32], i64>,
    remove_queue: VecDeque<[u8; 32]>,
    max_size: usize,
    time_to_live: i64,
    size: usize,
}

impl TxCache {
    fn new(max_size: usize, time_to_live: i64) -> Self {
        Self {
            cache: HashSet::new(),
            time_added: HashMap::new(),
            remove_queue: VecDeque::with_capacity(max_size),
            max_size,
            time_to_live,
            size: 0,
        }
    }

    fn cached(&self, tx_hash: [u8; 32]) -> bool {
        // the tx is known and entry hasn't expired
        self.cache.contains(&tx_hash)
            && (Time::now().unix_timestamp()
                <= self.time_added[&tx_hash]
                    .checked_add(self.time_to_live)
                    .expect("overflowed ttl add"))
    }

    fn add(&mut self, tx_hash: [u8; 32]) {
        if self.cache.contains(&tx_hash) {
            // update time to live if already exists
            // note: this doesn't change the tx's position in the remove vector
            self.time_added
                .insert(tx_hash, Time::now().unix_timestamp());
            return;
        };
        if self.size == self.max_size {
            // make space for the new transaction by removing the oldest transaction
            let removed_tx = self
                .remove_queue
                .pop_front()
                .expect("cache should contain elements");
            self.cache.remove(&removed_tx);
            self.time_added.remove(&removed_tx);
        } else {
            self.size = self.size.checked_add(1).expect("cache size overflowed");
        }
        self.remove_queue.push_back(tx_hash);
        self.cache.insert(tx_hash);
        self.time_added
            .insert(tx_hash, Time::now().unix_timestamp());
    }
}

/// Mempool handles [`request::CheckTx`] abci requests.
//
/// It performs a stateless check of the given transaction,
/// returning a [`tendermint::v0_38::abci::response::CheckTx`].
#[derive(Clone)]
#[allow(clippy::struct_field_names)]
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
            tx_cache: Arc::new(RwLock::new(TxCache::new(CACHE_SIZE, CACHE_TTL))),
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

    if tx_cache.read().await.cached(tx_hash) {
        // transaction has already been seen, remove from cometbft mempool
        return response::CheckTx {
            code: AbciErrorCode::ALREADY_PROCESSED.into(),
            log: format!(
                "transaction already known to mempool, can re-add after {CACHE_TTL} seconds"
            ),
            info: AbciErrorCode::ALREADY_PROCESSED.to_string(),
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

#[cfg(test)]
mod test {
    use std::time::Duration;

    use tokio::time;

    use crate::service::mempool::TxCache;

    #[tokio::test]
    async fn tx_cache_size() {
        let mut tx_cache = TxCache::new(2, 60);

        let tx_0 = [0u8; 32];
        let tx_1 = [1u8; 32];
        let tx_2 = [2u8; 32];

        assert!(
            !tx_cache.cached(tx_0),
            "no transaction should be cached at first"
        );

        tx_cache.add(tx_0);
        assert!(
            tx_cache.cached(tx_0),
            "transaction was added, should be cached"
        );

        tx_cache.add(tx_1);
        tx_cache.add(tx_2);
        assert!(
            tx_cache.cached(tx_1),
            "second transaction was added, should be cached"
        );
        assert!(
            tx_cache.cached(tx_2),
            "third transaction was added, should be cached"
        );
        assert!(
            !tx_cache.cached(tx_0),
            "first transaction should not be cached"
        );
    }

    #[tokio::test]
    async fn tx_cache_ttl() {
        let mut tx_cache = TxCache::new(2, 1);

        let tx_0 = [0u8; 32];
        tx_cache.add(tx_0);
        assert!(tx_cache.cached(tx_0), "transaction was added, should exist");

        // pass time to expire transaction
        time::sleep(Duration::from_secs(2)).await;

        assert!(
            !tx_cache.cached(tx_0),
            "transaction expired, should not be cached"
        );
    }
}
