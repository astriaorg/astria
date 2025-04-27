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
use base64::{
    prelude::BASE64_STANDARD,
    Engine as _,
};
use cnidarium::{
    StateRead,
    Storage,
};
use futures::{
    Future,
    FutureExt,
};
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
        Mempool as AppMempool,
        RemovalReason,
    },
    metrics::Metrics,
};

#[cfg(test)]
mod tests;

impl From<RemovalReason> for response::CheckTx {
    fn from(removal_reason: RemovalReason) -> Self {
        match removal_reason {
            RemovalReason::Expired => error_response(
                AbciErrorCode::TRANSACTION_EXPIRED,
                "transaction expired in the app's mempool",
            ),
            RemovalReason::FailedPrepareProposal(err) => error_response(
                AbciErrorCode::TRANSACTION_FAILED_EXECUTION,
                format!("transaction failed execution: {err}"),
            ),
            RemovalReason::NonceStale => error_response(
                AbciErrorCode::INVALID_NONCE,
                "transaction removed from app mempool due to stale nonce",
            ),
            RemovalReason::LowerNonceInvalidated => error_response(
                AbciErrorCode::LOWER_NONCE_INVALIDATED,
                "transaction removed from app mempool due to lower nonce being invalidated",
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

// impl From<CheckedTransactionExecutionError> for response::CheckTx {
//     fn from(error: CheckedTransactionExecutionError) -> Self {
//         let abci_error_code = match &error {
//             CheckedTransactionExecutionError::TooLarge {
//                 ..
//             }
//             | CheckedTransactionExecutionError::ActionIndexOverflowed => {
//                 AbciErrorCode::TRANSACTION_TOO_LARGE
//             }
//             CheckedTransactionExecutionError::Decode(_)
//             | CheckedTransactionExecutionError::Convert(_)
//             | CheckedTransactionExecutionError::CheckedAction(
//                 CheckedActionExecutionError::ActionDisabled {
//                     ..
//                 }
//                 | CheckedActionExecutionError::FeeAssetIsNotAllowed {
//                     ..
//                 },
//             ) => AbciErrorCode::BAD_REQUEST,
//             CheckedTransactionExecutionError::ChainIdMismatch {
//                 ..
//             } => AbciErrorCode::INVALID_CHAIN_ID,
//             CheckedTransactionExecutionError::InvalidNonce {
//                 ..
//             }
//             | CheckedTransactionExecutionError::NonceOverflowed => AbciErrorCode::INVALID_NONCE,
//             CheckedTransactionExecutionError::CheckedAction(
//                 CheckedActionExecutionError::InitialCheck {
//                     ..
//                 }
//                 | CheckedActionExecutionError::MutableCheck {
//                     ..
//                 },
//             ) => AbciErrorCode::TRANSACTION_FAILED_CHECK_TX,
//             CheckedTransactionExecutionError::CheckedAction(
//                 CheckedActionExecutionError::Execution {
//                     ..
//                 }
//                 | CheckedActionExecutionError::InsufficientBalanceToPayFee {
//                     ..
//                 },
//             ) => AbciErrorCode::TRANSACTION_FAILED_EXECUTION,
//             CheckedTransactionExecutionError::CheckedAction(
//                 CheckedActionExecutionError::InternalError {
//                     ..
//                 },
//             )
//             | CheckedTransactionExecutionError::InternalError {
//                 ..
//             } => AbciErrorCode::INTERNAL_ERROR,
//         };
//
//         error_response(abci_error_code, error)
//     }
// }

#[expect(clippy::needless_pass_by_value, reason = "more ergonomic to call")]
fn error_response<T: ToString>(abci_error_code: AbciErrorCode, log: T) -> response::CheckTx {
    response::CheckTx {
        code: Code::Err(abci_error_code.value()),
        info: abci_error_code.info(),
        log: log.to_string(),
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
/// This function will error if:
/// - the transaction has been removed from the app's mempool (will throw error once)
/// - the transaction fails conversion to a checked transaction
/// - the transaction fails insertion into the mempool
///
/// The function will return a [`response::CheckTx`] with a status code of 0 if the transaction:
/// - Is already in the appside mempool and passes `CheckedTransaction::run_mutable_checks`, or
/// - Passes checks and insertion into the mempool is successful
#[instrument(skip_all)]
async fn handle_check_tx<S: StateRead>(
    req: request::CheckTx,
    state: S,
    mempool: &mut AppMempool,
    metrics: &'static Metrics,
) -> response::CheckTx {
    use sha2::Digest as _;

    let request::CheckTx {
        tx: tx_bytes,
        kind: check_tx_kind,
    } = req;

    if check_tx_kind == CheckTxKind::Recheck {
        let tx_id = TransactionId::new(sha2::Sha256::digest(&tx_bytes).into());
        if let Some(response) = recheck_tx(&tx_id, mempool, metrics).await {
            return response;
        }
    }

    let start = Instant::now();
    let checked_tx = match CheckedTransaction::new(tx_bytes, &state).await {
        Ok(checked_tx) => checked_tx,
        Err(error) => {
            let abci_error_code = match &error {
                CheckedTransactionInitialCheckError::TooLarge {
                    ..
                } => {
                    metrics.increment_check_tx_failed_tx_too_large();
                    AbciErrorCode::TRANSACTION_TOO_LARGE
                }
                CheckedTransactionInitialCheckError::Decode(_)
                | CheckedTransactionInitialCheckError::Convert(_) => {
                    metrics.increment_check_tx_failed_action_checks();
                    AbciErrorCode::BAD_REQUEST
                }
                CheckedTransactionInitialCheckError::ChainIdMismatch {
                    ..
                } => {
                    metrics.increment_check_tx_failed_action_checks();
                    AbciErrorCode::INVALID_CHAIN_ID
                }
                CheckedTransactionInitialCheckError::CheckedAction(_) => {
                    metrics.increment_check_tx_failed_action_checks();
                    AbciErrorCode::TRANSACTION_FAILED_CHECK_TX
                }
                CheckedTransactionInitialCheckError::InternalError {
                    ..
                } => {
                    metrics.increment_internal_logic_error();
                    AbciErrorCode::INTERNAL_ERROR
                }
            };
            return error_response(abci_error_code, error);
        }
    };
    metrics.record_check_tx_duration_seconds_check_actions(start.elapsed());

    if let Err(rsp) = insert_into_mempool(mempool, &state, checked_tx, metrics).await {
        return rsp;
    }

    metrics.set_transactions_in_mempool_total(mempool.len().await);

    response::CheckTx::default()
}

/// Re-checks the transaction in the appside mempool.
///
/// Returns a successful response if the given tx passes `CheckedTransaction::run_mutable_checks`.
/// Returns a failure response if:
///   - the tx is scheduled to be removed from the mempool
///   - the tx returns an error from `run_mutable_checks`.
/// Returns `None` if the tx is not in the mempool.
#[instrument(skip_all, fields(%tx_id))]
async fn recheck_tx(
    tx_id: &TransactionId,
    mempool: &AppMempool,
    metrics: &Metrics,
) -> Option<response::CheckTx> {
    let start = Instant::now();

    let rsp = match mempool.check_removed_comet_bft(tx_id).await {
        Some(removal_reason) => {
            match removal_reason {
                RemovalReason::Expired => {
                    metrics.increment_check_tx_removed_expired();
                }
                RemovalReason::FailedPrepareProposal(_) => {
                    metrics.increment_check_tx_removed_failed_execution();
                }
                _ => (),
            }
            Some(response::CheckTx::from(removal_reason))
        }
        None => {
            if mempool.is_tracked(tx_id).await {
                Some(response::CheckTx::default())
            } else {
                None
            }
        }
    };

    metrics.record_check_tx_duration_seconds_recheck(start.elapsed());

    rsp
}

/// Attempts to insert the transaction into the mempool.
#[instrument(skip_all)]
async fn insert_into_mempool<S: StateRead>(
    mempool: &AppMempool,
    state: &S,
    tx: CheckedTransaction,
    metrics: &'static Metrics,
) -> Result<(), response::CheckTx> {
    let address_bytes = *tx.address_bytes();

    // fetch current account nonce
    let start_fetch_nonce = Instant::now();
    let current_account_nonce = state
        .get_account_nonce(&address_bytes)
        .await
        .map_err(|error| {
            error_response(
                AbciErrorCode::INTERNAL_ERROR,
                format!(
                    "failed to get nonce for account `{}` from storage: {error:#}",
                    BASE64_STANDARD.encode(address_bytes)
                ),
            )
        })?;

    let finished_fetch_nonce = Instant::now();
    metrics.record_check_tx_duration_seconds_fetch_nonce(
        finished_fetch_nonce.saturating_duration_since(start_fetch_nonce),
    );

    // grab cost of transaction
    let transaction_cost = tx.total_costs(state).await.map_err(|error| {
        error_response(
            AbciErrorCode::INTERNAL_ERROR,
            format!("failed to calculate cost of the transaction: {error:#}"),
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
                error_response(
                    AbciErrorCode::INTERNAL_ERROR,
                    format!(
                        "failed to get balances for account `{}` from storage: {error:#}",
                        BASE64_STANDARD.encode(address_bytes)
                    ),
                )
            })?;

    let finished_fetch_balances = Instant::now();
    metrics.record_check_tx_duration_seconds_fetch_balances(
        finished_fetch_balances.saturating_duration_since(finished_fetch_tx_cost),
    );

    let actions_count = tx.checked_actions().len();
    let tx_length = tx.encoded_bytes().len();

    mempool
        .insert(
            Arc::new(tx),
            current_account_nonce,
            current_account_balances,
            transaction_cost,
        )
        .await?;

    metrics
        .record_check_tx_duration_seconds_insert_to_app_mempool(finished_fetch_balances.elapsed());
    metrics.record_actions_per_transaction_in_mempool(actions_count);
    metrics.record_transaction_in_mempool_size_bytes(tx_length);

    Ok(())
}
