use std::{
    pin::Pin,
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

/// Mempool handles [`request::CheckTx`] abci requests.
//
/// It performs a stateless check of the given transaction,
/// returning a [`tendermint::v0_38::abci::response::CheckTx`].
#[derive(Clone)]
pub(crate) struct Mempool {
    storage: Storage,
    mempool: AppMempool,
}

impl Mempool {
    pub(crate) fn new(storage: Storage, mempool: AppMempool) -> Self {
        Self {
            storage,
            mempool,
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
        async move {
            let rsp = match req {
                MempoolRequest::CheckTx(req) => MempoolResponse::CheckTx(
                    handle_check_tx(req, storage.latest_snapshot(), &mut mempool).await,
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
) -> response::CheckTx {
    use astria_core::primitive::v1::Address;
    use sha2::Digest as _;

    let tx_hash = sha2::Sha256::digest(&req.tx).into();

    let request::CheckTx {
        tx, ..
    } = req;
    if tx.len() > MAX_TX_SIZE {
        mempool.remove(&tx_hash).await;
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
            mempool.remove(&tx_hash).await;
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
            mempool.remove(&tx_hash).await;
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
        mempool.remove(&tx_hash).await;
        metrics::counter!(metrics_init::CHECK_TX_REMOVED_FAILED_STATELESS).increment(1);
        return response::CheckTx {
            code: AbciErrorCode::INVALID_PARAMETER.into(),
            info: "transaction failed stateless check".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    if let Err(e) = transaction::check_nonce_mempool(&signed_tx, &state).await {
        mempool.remove(&tx_hash).await;
        metrics::counter!(metrics_init::CHECK_TX_REMOVED_STALE_NONCE).increment(1);
        return response::CheckTx {
            code: AbciErrorCode::INVALID_NONCE.into(),
            info: "failed verifying transaction nonce".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    if let Err(e) = transaction::check_chain_id_mempool(&signed_tx, &state).await {
        mempool.remove(&tx_hash).await;
        return response::CheckTx {
            code: AbciErrorCode::INVALID_CHAIN_ID.into(),
            info: "failed verifying chain id".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    }

    if let Err(e) = transaction::check_balance_mempool(&signed_tx, &state).await {
        mempool.remove(&tx_hash).await;
        metrics::counter!(metrics_init::CHECK_TX_REMOVED_ACCOUNT_BALANCE).increment(1);
        return response::CheckTx {
            code: AbciErrorCode::INSUFFICIENT_FUNDS.into(),
            info: "failed verifying account balance".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    // tx is valid, push to mempool
    let priority = crate::mempool::TransactionPriority::new(
        signed_tx.nonce(),
        state
            .get_account_nonce(Address::from_verification_key(signed_tx.verification_key()))
            .await
            .expect("can fetch account nonce"),
    )
    .expect(
        "tx nonce is greater or equal to current account nonce; this was checked in \
         check_nonce_mempool",
    );
    tracing::info!("inserting tx into mempool");
    mempool
        .insert(signed_tx, priority)
        .await
        .expect("priority transaction nonce and transaction nonce match, as we set them above");
    tracing::info!("inserted tx into mempool");
    response::CheckTx::default()
}
