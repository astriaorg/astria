use std::{
    pin::Pin,
    sync::Arc,
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
    FutureExt as _,
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
    accounts::StateReadExt as _,
    app::App,
    mempool::RemovalReason,
    metrics::Metrics,
    transaction::InvalidNonce,
};

#[cfg(test)]
mod tests;

impl<'a> From<&'a crate::app::TransactionTooLarge> for response::CheckTx {
    fn from(value: &'a crate::app::TransactionTooLarge) -> Self {
        response::CheckTx {
            code: AbciErrorCode::TRANSACTION_TOO_LARGE.into(),
            info: AbciErrorCode::TRANSACTION_TOO_LARGE.to_string(),
            log: format!("transaction failed execution because: {value:#?}"),
            ..response::CheckTx::default()
        }
    }
}

impl<'a> From<&'a RemovalReason> for response::CheckTx {
    fn from(value: &'a RemovalReason) -> Self {
        let code = match value {
            RemovalReason::Expired => AbciErrorCode::TRANSACTION_EXPIRED,
            RemovalReason::FailedPrepareProposal {
                ..
            } => AbciErrorCode::TRANSACTION_FAILED,
        };
        response::CheckTx {
            code: code.into(),
            info: code.to_string(),
            log: format!("transaction failed execution because: {value:#?}"),
            ..response::CheckTx::default()
        }
    }
}

fn dynamic_error_to_abci_response(
    err: &anyhow::Error,
    metrics: &'static Metrics,
) -> response::CheckTx {
    if let Some(err) = err.downcast_ref::<crate::app::TransactionTooLarge>() {
        metrics.increment_check_tx_removed_too_large();
        return err.into();
    }
    if let Some(err) = err.downcast_ref::<RemovalReason>() {
        match &err {
            RemovalReason::Expired => metrics.increment_check_tx_removed_expired(),
            RemovalReason::FailedPrepareProposal {
                ..
            } => metrics.increment_check_tx_removed_failed_execution(),
        }
        return err.into();
    }
    // FIXME: this is used as a catch-all right now, even though "internal error"
    //        might be misleading or wrong. Need to figure out how to map the
    //        currently opaque tx.check_and_execute to specific abci error codes.
    metrics.increment_check_tx_removed_failed_speculative_deliver_tx();
    response::CheckTx {
        code: AbciErrorCode::INTERNAL_ERROR.into(),
        info: AbciErrorCode::INTERNAL_ERROR.to_string(),
        log: format!("transaction failed execution because: {err:#?}"),
        ..response::CheckTx::default()
    }
}

/// Mempool handles [`request::CheckTx`] abci requests.
//
/// It performs a stateless check of the given transaction,
/// returning a [`tendermint::v0_38::abci::response::CheckTx`].
#[derive(Clone)]
pub(crate) struct Mempool {
    storage: Storage,
    inner: crate::mempool::Mempool,
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
        let app_mempool = self.inner.clone();
        let metrics = self.metrics;
        async move {
            let rsp = match req {
                MempoolRequest::CheckTx(req) => MempoolResponse::CheckTx(
                    handle_check_tx(req, storage, app_mempool, metrics).await,
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

    let finished_speculative_deliver_tx = Instant::now();
    let snapshot = storage.latest_snapshot();
    let mut app = App::new(snapshot.clone(), mempool.clone(), metrics)
        .await
        .unwrap();

    let (the_tx, _) = match app.deliver_tx_bytes(&bytes).await {
        Err(mut err) => {
            if let Some(current_nonce) = find_invalid_nonce_error(&err)
                .and_then(|invalid_nonce| invalid_nonce.is_ahead().then_some(invalid_nonce.current))
            {
                let signed_tx = transaction_from_bytes_unchecked(&bytes);
                if let Err(mempool_error) = mempool
                    .insert(Arc::new(signed_tx), current_nonce)
                    .await
                    .context("mempool rejected transaction with future nonce")
                {
                    // override the outer arror and fall down to the general handler
                    err = mempool_error;
                } else {
                    return response::CheckTx::default();
                }
            }
            return dynamic_error_to_abci_response(&err, metrics);
        }
        Ok(ret) => ret,
    };

    metrics.record_check_tx_duration_seconds_speculative_deliver_tx(
        finished_speculative_deliver_tx.elapsed(),
    );

    if let Some(removal_reason) = mempool.check_removed_comet_bft(tx_hash).await {
        mempool.remove(tx_hash).await;
        return dynamic_error_to_abci_response(&anyhow::Error::new(removal_reason), metrics);
    };

    let finished_check_removed = Instant::now();
    metrics.record_check_tx_duration_seconds_check_removed(
        finished_check_removed.saturating_duration_since(finished_speculative_deliver_tx),
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

    if let Err(err) = mempool
        .insert(the_tx.clone(), current_account_nonce)
        .await
        .context("mempool rejected validated transaction")
    {
        return response::CheckTx {
            code: AbciErrorCode::INTERNAL_ERROR.into(),
            info: AbciErrorCode::INTERNAL_ERROR.to_string(),
            log: format!("transaction failed execution because: {err:#?}"),
            ..response::CheckTx::default()
        };
    }
    let mempool_len = mempool.len().await;

    metrics
        .record_check_tx_duration_seconds_insert_to_app_mempool(finished_check_removed.elapsed());
    metrics.record_actions_per_transaction_in_mempool(the_tx.actions().len());
    metrics.record_transaction_in_mempool_size_bytes(bytes.len());
    metrics.set_transactions_in_mempool_total(mempool_len);

    response::CheckTx::default()
}

fn find_invalid_nonce_error(error: &anyhow::Error) -> Option<&InvalidNonce> {
    for cause in error.chain() {
        if let Some(invalid_nonce) = cause.downcast_ref::<InvalidNonce>() {
            return Some(invalid_nonce);
        }
    }
    None
}

/// Constructs a signed transaction from bytes, panicking if decoding
/// the protobuf bytes failed or if the transaction was malformed.
fn transaction_from_bytes_unchecked(bytes: &[u8]) -> SignedTransaction {
    let proto = raw::SignedTransaction::decode(bytes).expect(
        "an invalid nonce was established which only makes sense if the transaction was \
         successfully decoded",
    );
    SignedTransaction::try_from_raw(proto).expect(
        "an invalid nonce was established which only makes sense if the transaction was well \
         formed",
    )
}
