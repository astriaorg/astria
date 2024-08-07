use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
    time::Instant,
};

use anyhow::Context as _;
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
    TryFutureExt as _,
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
use tracing::{
    instrument,
    Instrument as _,
};

use crate::{
    accounts,
    address,
    app::ActionHandler as _,
    mempool::{
        Mempool as AppMempool,
        RemovalReason,
    },
    metrics::Metrics,
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
    inner: AppMempool,
    metrics: &'static Metrics,
}

impl Mempool {
    pub(crate) fn new(storage: Storage, mempool: AppMempool, metrics: &'static Metrics) -> Self {
        Self {
            storage,
            inner: mempool,
            metrics,
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
        let mut mempool = self.inner.clone();
        let metrics = self.metrics;
        async move {
            let rsp = match req {
                MempoolRequest::CheckTx(req) => MempoolResponse::CheckTx(
                    handle_check_tx(req, storage.latest_snapshot(), &mut mempool, metrics).await,
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
#[instrument(skip_all)]
async fn handle_check_tx<S: accounts::StateReadExt + address::StateReadExt + 'static>(
    req: request::CheckTx,
    state: S,
    mempool: &mut AppMempool,
    metrics: &'static Metrics,
) -> response::CheckTx {
    use sha2::Digest as _;

    let start_parsing = Instant::now();

    let request::CheckTx {
        tx, ..
    } = req;

    let tx_hash = sha2::Sha256::digest(&tx).into();
    let tx_len = tx.len();

    if tx_len > MAX_TX_SIZE {
        mempool.remove(tx_hash).await;
        metrics.increment_check_tx_removed_too_large();
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

    let finished_parsing = Instant::now();
    metrics.record_check_tx_duration_seconds_parse_tx(
        finished_parsing.saturating_duration_since(start_parsing),
    );

    if let Err(e) = signed_tx.check_stateless(()).await {
        mempool.remove(tx_hash).await;
        metrics.increment_check_tx_removed_failed_stateless();
        return response::CheckTx {
            code: AbciErrorCode::INVALID_PARAMETER.into(),
            info: "transaction failed stateless check".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    let finished_check_stateless = Instant::now();
    metrics.record_check_tx_duration_seconds_check_stateless(
        finished_check_stateless.saturating_duration_since(finished_parsing),
    );

    if let Err(e) = transaction::check_nonce_mempool(&signed_tx, &state).await {
        mempool.remove(tx_hash).await;
        metrics.increment_check_tx_removed_stale_nonce();
        return response::CheckTx {
            code: AbciErrorCode::INVALID_NONCE.into(),
            info: "failed verifying transaction nonce".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    let finished_check_nonce = Instant::now();
    metrics.record_check_tx_duration_seconds_check_nonce(
        finished_check_nonce.saturating_duration_since(finished_check_stateless),
    );

    if let Err(e) = transaction::check_chain_id_mempool(&signed_tx, &state).await {
        mempool.remove(tx_hash).await;
        return response::CheckTx {
            code: AbciErrorCode::INVALID_CHAIN_ID.into(),
            info: "failed verifying chain id".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    }

    let finished_check_chain_id = Instant::now();
    metrics.record_check_tx_duration_seconds_check_chain_id(
        finished_check_chain_id.saturating_duration_since(finished_check_nonce),
    );

    if let Err(e) = transaction::check_balance_mempool(&signed_tx, &state).await {
        mempool.remove(tx_hash).await;
        metrics.increment_check_tx_removed_account_balance();
        return response::CheckTx {
            code: AbciErrorCode::INSUFFICIENT_FUNDS.into(),
            info: "failed verifying account balance".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    let finished_check_balance = Instant::now();
    metrics.record_check_tx_duration_seconds_check_balance(
        finished_check_balance.saturating_duration_since(finished_check_chain_id),
    );

    if let Some(removal_reason) = mempool.check_removed_comet_bft(tx_hash).await {
        mempool.remove(tx_hash).await;

        match removal_reason {
            RemovalReason::Expired => {
                metrics.increment_check_tx_removed_expired();
                return response::CheckTx {
                    code: AbciErrorCode::TRANSACTION_EXPIRED.into(),
                    info: "transaction expired in app's mempool".into(),
                    log: "Transaction expired in the app's mempool".into(),
                    ..response::CheckTx::default()
                };
            }
            RemovalReason::FailedPrepareProposal(err) => {
                metrics.increment_check_tx_removed_failed_execution();
                return response::CheckTx {
                    code: AbciErrorCode::TRANSACTION_FAILED.into(),
                    info: "transaction failed execution in prepare_proposal()".into(),
                    log: format!("transaction failed execution because: {err}"),
                    ..response::CheckTx::default()
                };
            }
        }
    };

    let finished_check_removed = Instant::now();
    metrics.record_check_tx_duration_seconds_check_removed(
        finished_check_removed.saturating_duration_since(finished_check_balance),
    );

    // tx is valid, push to mempool
    let current_account_nonce = match state
        .try_base_prefixed(&signed_tx.verification_key().address_bytes())
        .and_then(|address| state.get_account_nonce(address))
        .await
        .context("failed fetching nonce for account")
    {
        Err(err) => {
            return response::CheckTx {
                code: AbciErrorCode::INTERNAL_ERROR.into(),
                info: AbciErrorCode::INTERNAL_ERROR.to_string(),
                log: format!("transaction failed execution because: {err:#?}"),
                ..response::CheckTx::default()
            };
        }
        Ok(nonce) => nonce,
    };

    let actions_count = signed_tx.actions().len();

    mempool
        .insert(signed_tx, current_account_nonce)
        .await
        .expect(
            "tx nonce is greater than or equal to current account nonce; this was checked in \
             check_nonce_mempool",
        );
    let mempool_len = mempool.len().await;

    metrics
        .record_check_tx_duration_seconds_insert_to_app_mempool(finished_check_removed.elapsed());
    metrics.record_actions_per_transaction_in_mempool(actions_count);
    metrics.record_transaction_in_mempool_size_bytes(tx_len);
    metrics.set_transactions_in_mempool_total(mempool_len);

    response::CheckTx::default()
}
