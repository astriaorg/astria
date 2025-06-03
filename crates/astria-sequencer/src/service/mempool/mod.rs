use std::{
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
    },
    time::Instant,
};

use astria_core::{
    primitive::v1::TransactionId,
    protocol::abci::AbciErrorCode,
};
use astria_eyre::eyre::Report;
use base64::{
    prelude::BASE64_STANDARD,
    Engine as _,
};
use bytes::Bytes;
use cnidarium::{
    StateRead,
    Storage,
};
use futures::{
    Future,
    FutureExt,
};
use sha2::Digest as _;
use tendermint::{
    abci::{
        request::CheckTxKind,
        Code,
    },
    v0_38::abci::{
        request,
        response,
        MempoolRequest,
        MempoolResponse,
    },
};
use tower::Service;
use tower_abci::BoxError;
use tracing::{
    instrument,
    Instrument as _,
};

use crate::{
    accounts::{
        AddressBytes as _,
        StateReadExt as _,
    },
    checked_transaction::{
        CheckedTransaction,
        CheckedTransactionInitialCheckError,
    },
    mempool::{
        get_account_balances,
        InsertionError,
        InsertionStatus,
        Mempool as AppMempool,
        RemovalReason,
        TransactionStatus,
    },
    metrics::Metrics,
};

#[cfg(test)]
mod tests;

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
        let mempool = self.inner.clone();
        let metrics = self.metrics;
        async move {
            let rsp = match req {
                MempoolRequest::CheckTx(req) => MempoolResponse::CheckTx(
                    handle_check_tx_request(req, storage.latest_snapshot(), &mempool, metrics)
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
/// This function will error if:
/// - the transaction has been removed from the app's mempool (will throw error once)
/// - the transaction fails conversion to a checked transaction
/// - the transaction fails insertion into the mempool
///
/// The function will return a [`response::CheckTx`] with a status code of 0 if the transaction:
/// - is already in the appside mempool, or
/// - passes checks and insertion into the mempool is successful
#[instrument(skip_all)]
async fn handle_check_tx_request<S: StateRead>(
    req: request::CheckTx,
    state: S,
    mempool: &AppMempool,
    metrics: &'static Metrics,
) -> response::CheckTx {
    let request::CheckTx {
        tx: tx_bytes,
        kind: check_tx_kind,
    } = req;

    let start = Instant::now();

    let outcome = check_tx(tx_bytes, state, mempool, metrics).await;
    let response = if let CheckTxOutcome::RemovedFromMempool {
        tx_id,
        reason,
    } = outcome
    {
        match reason {
            RemovalReason::Expired => {
                metrics.increment_check_tx_removed_expired();
            }
            RemovalReason::FailedPrepareProposal(_) => {
                metrics.increment_check_tx_removed_failed_execution();
            }
            _ => {}
        };
        mempool.remove_from_removal_cache(&tx_id).await;
        reason.into()
    } else {
        outcome.into()
    };

    if check_tx_kind == CheckTxKind::Recheck {
        metrics.record_check_tx_duration_seconds_recheck(start.elapsed());
    }

    response
}

/// Performs `CheckTx` for a given serialized [`Transaction`]. This consists of checking that the
/// transaction is not already in the app-side mempool or has been removed from it, performing
/// checks required to convert to a [`CheckedTransaction`], and inserting the transaction into the
/// mempool.
///
/// Returns a [`CheckTxOutcome`] indicating the result of the operation.
pub(crate) async fn check_tx<S: StateRead>(
    tx_bytes: Bytes,
    state: S,
    mempool: &AppMempool,
    metrics: &'static Metrics,
) -> CheckTxOutcome {
    let tx_status_start = Instant::now();
    let tx_id = TransactionId::new(sha2::Sha256::digest(&tx_bytes).into());
    let outcome = match mempool.transaction_status(&tx_id).await {
        Some(TransactionStatus::Parked) => Some(CheckTxOutcome::AlreadyInParked(tx_id)),
        Some(TransactionStatus::Pending) => Some(CheckTxOutcome::AlreadyInPending(tx_id)),
        Some(TransactionStatus::Removed(reason)) => Some(CheckTxOutcome::RemovedFromMempool {
            tx_id,
            reason,
        }),
        None => None,
    };
    let tx_status_end = Instant::now();
    metrics.record_check_tx_duration_seconds_transaction_status(
        tx_status_end.saturating_duration_since(tx_status_start),
    );
    if let Some(outcome) = outcome {
        return outcome;
    }

    let checked_tx = match CheckedTransaction::new(tx_bytes, &state).await {
        Ok(checked_tx) => checked_tx,
        Err(error) => {
            match &error {
                CheckedTransactionInitialCheckError::TooLarge {
                    ..
                } => {
                    metrics.increment_check_tx_failed_tx_too_large();
                }
                CheckedTransactionInitialCheckError::Decode(_)
                | CheckedTransactionInitialCheckError::Convert(_)
                | CheckedTransactionInitialCheckError::InvalidNonce {
                    ..
                }
                | CheckedTransactionInitialCheckError::ChainIdMismatch {
                    ..
                }
                | CheckedTransactionInitialCheckError::CheckedAction(_) => {
                    metrics.increment_check_tx_failed_action_checks();
                }
                CheckedTransactionInitialCheckError::InternalError {
                    ..
                } => {
                    metrics.increment_internal_logic_error();
                }
            }
            return CheckTxOutcome::FailedChecks(error);
        }
    };
    metrics.record_check_tx_duration_seconds_check_actions(tx_status_end.elapsed());

    // attempt to insert the transaction into the mempool
    let insertion_status = match insert_into_mempool(mempool, &state, checked_tx, metrics).await {
        Ok(status) => status,
        Err(outcome) => {
            return outcome;
        }
    };

    metrics.set_transactions_in_mempool_total(mempool.len().await);

    match insertion_status {
        InsertionStatus::AddedToParked => CheckTxOutcome::AddedToParked(tx_id),
        InsertionStatus::AddedToPending => CheckTxOutcome::AddedToPending(tx_id),
    }
}

/// Attempts to insert the transaction into the mempool.
#[instrument(skip_all)]
async fn insert_into_mempool<S: StateRead>(
    mempool: &AppMempool,
    state: &S,
    tx: CheckedTransaction,
    metrics: &'static Metrics,
) -> Result<InsertionStatus, CheckTxOutcome> {
    let address_bytes = *tx.address_bytes();

    // fetch current account nonce
    let start_fetch_nonce = Instant::now();
    let current_account_nonce = state
        .get_account_nonce(&address_bytes)
        .await
        .map_err(|error| {
            CheckTxOutcome::InternalError(error.wrap_err(format!(
                "failed to get nonce for account `{}` from storage",
                BASE64_STANDARD.encode(address_bytes)
            )))
        })?;

    let finished_fetch_nonce = Instant::now();
    metrics.record_check_tx_duration_seconds_fetch_nonce(
        finished_fetch_nonce.saturating_duration_since(start_fetch_nonce),
    );

    // grab cost of transaction
    let transaction_costs = tx.total_costs(state).await.map_err(|error| {
        CheckTxOutcome::InternalError(
            Report::new(error).wrap_err("failed to calculate cost of the transaction"),
        )
    })?;

    let finished_fetch_tx_cost = Instant::now();
    metrics.record_check_tx_duration_seconds_fetch_tx_cost(
        finished_fetch_tx_cost.saturating_duration_since(finished_fetch_nonce),
    );

    // grab current account's balances
    let current_account_balances =
        get_account_balances(&state, &address_bytes)
            .await
            .map_err(|error| {
                CheckTxOutcome::InternalError(error.wrap_err(format!(
                    "failed to get balances for account `{}` from storage",
                    BASE64_STANDARD.encode(address_bytes)
                )))
            })?;

    let finished_fetch_balances = Instant::now();
    metrics.record_check_tx_duration_seconds_fetch_balances(
        finished_fetch_balances.saturating_duration_since(finished_fetch_tx_cost),
    );

    let actions_count = tx.checked_actions().len();
    let tx_length = tx.encoded_bytes().len();

    let insertion_status = mempool
        .insert(
            Arc::new(tx),
            current_account_nonce,
            &current_account_balances,
            transaction_costs,
        )
        .await
        .map_err(CheckTxOutcome::FailedInsertion)?;

    metrics
        .record_check_tx_duration_seconds_insert_to_app_mempool(finished_fetch_balances.elapsed());
    metrics.record_actions_per_transaction_in_mempool(actions_count);
    metrics.record_transaction_in_mempool_size_bytes(tx_length);

    Ok(insertion_status)
}

#[derive(Debug)]
pub(crate) enum CheckTxOutcome {
    AddedToPending(TransactionId),
    AddedToParked(TransactionId),
    AlreadyInPending(TransactionId),
    AlreadyInParked(TransactionId),
    FailedChecks(CheckedTransactionInitialCheckError),
    FailedInsertion(InsertionError),
    RemovedFromMempool {
        tx_id: TransactionId,
        reason: RemovalReason,
    },
    InternalError(Report),
}

impl From<RemovalReason> for response::CheckTx {
    fn from(removal_reason: RemovalReason) -> Self {
        let log = format!("transaction removed from app mempool: {removal_reason}");
        let code = match removal_reason {
            RemovalReason::Expired => AbciErrorCode::TRANSACTION_EXPIRED,
            RemovalReason::NonceStale => AbciErrorCode::INVALID_NONCE,
            RemovalReason::LowerNonceInvalidated => AbciErrorCode::LOWER_NONCE_INVALIDATED,
            RemovalReason::FailedPrepareProposal(_) => AbciErrorCode::TRANSACTION_FAILED_EXECUTION,
            RemovalReason::IncludedInBlock {
                ..
            } => AbciErrorCode::TRANSACTION_INCLUDED_IN_BLOCK,
            RemovalReason::InternalError => AbciErrorCode::INTERNAL_ERROR,
        };
        error_response(code, log)
    }
}

impl From<CheckedTransactionInitialCheckError> for response::CheckTx {
    fn from(error: CheckedTransactionInitialCheckError) -> Self {
        let log = format!("transaction failed initial checks: {error}");
        let code = match error {
            CheckedTransactionInitialCheckError::TooLarge {
                ..
            } => AbciErrorCode::TRANSACTION_TOO_LARGE,
            CheckedTransactionInitialCheckError::Decode(_) => {
                AbciErrorCode::INVALID_TRANSACTION_BYTES
            }
            CheckedTransactionInitialCheckError::Convert(_) => AbciErrorCode::INVALID_TRANSACTION,
            CheckedTransactionInitialCheckError::InvalidNonce {
                ..
            } => AbciErrorCode::INVALID_NONCE,
            CheckedTransactionInitialCheckError::ChainIdMismatch {
                ..
            } => AbciErrorCode::INVALID_CHAIN_ID,
            CheckedTransactionInitialCheckError::CheckedAction(_) => {
                AbciErrorCode::TRANSACTION_FAILED_CHECK_TX
            }
            CheckedTransactionInitialCheckError::InternalError {
                ..
            } => AbciErrorCode::INTERNAL_ERROR,
        };
        error_response(code, log)
    }
}

impl From<CheckTxOutcome> for response::CheckTx {
    fn from(outcome: CheckTxOutcome) -> Self {
        match outcome {
            CheckTxOutcome::AddedToParked(_)
            | CheckTxOutcome::AddedToPending(_)
            | CheckTxOutcome::AlreadyInParked(_)
            | CheckTxOutcome::AlreadyInPending(_) => response::CheckTx::default(),
            CheckTxOutcome::FailedChecks(error) => error.into(),
            CheckTxOutcome::FailedInsertion(error) => error.into(),
            CheckTxOutcome::RemovedFromMempool {
                reason, ..
            } => reason.into(),
            CheckTxOutcome::InternalError(report) => error_response(
                AbciErrorCode::INTERNAL_ERROR,
                format!("internal error: {report}"),
            ),
        }
    }
}

impl From<InsertionError> for response::CheckTx {
    fn from(insertion_error: InsertionError) -> Self {
        let log = format!("failed to insert transaction into app mempool: {insertion_error}");
        let code = match insertion_error {
            InsertionError::AlreadyPresent => AbciErrorCode::ALREADY_PRESENT,
            InsertionError::NonceTooLow => AbciErrorCode::INVALID_NONCE,
            InsertionError::NonceTaken => AbciErrorCode::NONCE_TAKEN,
            InsertionError::NonceGap | InsertionError::AccountBalanceTooLow => {
                AbciErrorCode::INTERNAL_ERROR
            }
            InsertionError::AccountSizeLimit => AbciErrorCode::ACCOUNT_SIZE_LIMIT,
            InsertionError::ParkedSizeLimit => AbciErrorCode::PARKED_FULL,
        };
        error_response(code, log)
    }
}

#[expect(clippy::needless_pass_by_value, reason = "more ergonomic to call")]
fn error_response<T: ToString>(abci_error_code: AbciErrorCode, log: T) -> response::CheckTx {
    response::CheckTx {
        code: Code::Err(abci_error_code.value()),
        info: abci_error_code.info(),
        log: log.to_string(),
        ..response::CheckTx::default()
    }
}
