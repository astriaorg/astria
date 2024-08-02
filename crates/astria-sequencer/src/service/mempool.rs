use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
    time::Instant,
};

use anyhow::Context as _;
use astria_core::protocol::abci::AbciErrorCode;
use cnidarium::Storage;
use futures::{
    Future,
    FutureExt as _,
};
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
    accounts::StateReadExt as _,
    app::App,
    mempool::RemovalReason,
    metrics::Metrics,
};

const MAX_TX_SIZE: usize = 256_000; // 256 KB

/// Mempool handles [`request::CheckTx`] abci requests.
//
/// It performs a stateless check of the given transaction,
/// returning a [`tendermint::v0_38::abci::response::CheckTx`].
#[derive(Clone)]
pub(crate) struct Mempool {
    storage: Storage,
    mempool: crate::mempool::Mempool,
    metrics: &'static Metrics,
}

impl Mempool {
    pub(crate) fn new(
        storage: Storage,
        mempool: crate::mempool::Mempool,
        metrics: &'static Metrics,
    ) -> Self {
        Self {
            storage,
            mempool,
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
        let mempool = self.mempool.clone();
        let metrics = self.metrics;
        async move {
            let rsp = match req {
                MempoolRequest::CheckTx(req) => {
                    MempoolResponse::CheckTx(handle_check_tx(req, storage, mempool, metrics).await)
                }
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
async fn handle_check_tx(
    req: request::CheckTx,
    storage: Storage,
    mempool: crate::mempool::Mempool,
    metrics: &'static Metrics,
) -> response::CheckTx {
    use sha2::Digest as _;

    let request::CheckTx {
        tx: bytes, ..
    } = req;

    let tx_hash = sha2::Sha256::digest(&bytes).into();

    // FIXME: this might be a good candidate to move to `App::execute_transaction_bytes`.
    if bytes.len() > MAX_TX_SIZE {
        mempool.remove(tx_hash).await;
        metrics.increment_check_tx_removed_too_large();
        return response::CheckTx {
            code: AbciErrorCode::TRANSACTION_TOO_LARGE.into(),
            log: format!(
                "transaction size too large; allowed: {MAX_TX_SIZE} bytes, got {}",
                bytes.len()
            ),
            info: AbciErrorCode::TRANSACTION_TOO_LARGE.to_string(),
            ..response::CheckTx::default()
        };
    }

    let finished_check_and_execute = Instant::now();
    let snapshot = storage.latest_snapshot();
    let mut app = App::new(snapshot.clone(), mempool.clone(), metrics)
        .await
        .unwrap();

    let (the_tx, _) = app.execute_transaction_bytes(&bytes).await.unwrap();

    metrics
        .record_check_tx_duration_seconds_check_and_execute(finished_check_and_execute.elapsed());

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
        finished_check_removed.saturating_duration_since(finished_check_and_execute),
    );

    // tx is valid, push to mempool
    let current_account_nonce = match snapshot
        .get_account_nonce(the_tx.address_bytes())
        .await
        .context("failed fetching nonce for transaction signer")
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

    mempool
        .insert(the_tx.clone(), current_account_nonce)
        .await
        .expect(
            "tx nonce is greater than or equal to current account nonce; this was checked in \
             check_nonce_mempool",
        );
    let mempool_len = mempool.len().await;

    metrics
        .record_check_tx_duration_seconds_insert_to_app_mempool(finished_check_removed.elapsed());
    metrics.record_actions_per_transaction_in_mempool(the_tx.actions().len());
    metrics.record_transaction_in_mempool_size_bytes(bytes.len());
    metrics.set_transactions_in_mempool_total(mempool_len);

    response::CheckTx::default()
}
