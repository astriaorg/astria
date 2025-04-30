use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
    },
    time::Instant,
};

use astria_core::{
    generated::astria::protocol::transaction::v1 as raw,
    primitive::v1::asset::IbcPrefixed,
    protocol::{
        abci::AbciErrorCode,
        transaction::v1::Transaction,
    },
    Protobuf as _,
};
use astria_eyre::eyre::WrapErr as _;
use bytes::Bytes;
use cnidarium::{
    StateRead,
    Storage,
};
pub(crate) use error::CheckTxError;
use futures::{
    Future,
    FutureExt,
};
use prost::{
    Message as _,
    Name as _,
};
use tendermint::{
    abci::Code,
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
    accounts::StateReadExt as _,
    action_handler::ActionHandler as _,
    address::StateReadExt as _,
    app::StateReadExt as _,
    mempool::{
        get_account_balances,
        InsertionError,
        Mempool as AppMempool,
        RemovalReason,
        TransactionStatus,
    },
    metrics::Metrics,
    transaction,
};

mod error;
#[cfg(test)]
mod tests;

const MAX_TX_SIZE: usize = 256_000; // 256 KB

pub(crate) trait IntoCheckTxResponse {
    fn into_check_tx_response(self) -> response::CheckTx;
}

impl IntoCheckTxResponse for RemovalReason {
    fn into_check_tx_response(self) -> response::CheckTx {
        match self {
            RemovalReason::Expired => response::CheckTx {
                code: Code::Err(AbciErrorCode::TRANSACTION_EXPIRED.value()),
                info: AbciErrorCode::TRANSACTION_EXPIRED.to_string(),
                log: "transaction expired in the app's mempool".into(),
                ..response::CheckTx::default()
            },
            RemovalReason::FailedPrepareProposal(err) => response::CheckTx {
                code: Code::Err(AbciErrorCode::TRANSACTION_FAILED.value()),
                info: AbciErrorCode::TRANSACTION_FAILED.to_string(),
                log: format!("transaction failed execution because: {err}"),
                ..response::CheckTx::default()
            },
            RemovalReason::NonceStale => response::CheckTx {
                code: Code::Err(AbciErrorCode::INVALID_NONCE.value()),
                info: "transaction removed from app mempool due to stale nonce".into(),
                log: "transaction from app mempool due to stale nonce".into(),
                ..response::CheckTx::default()
            },
            RemovalReason::LowerNonceInvalidated => response::CheckTx {
                code: Code::Err(AbciErrorCode::LOWER_NONCE_INVALIDATED.value()),
                info: AbciErrorCode::LOWER_NONCE_INVALIDATED.to_string(),
                log: "transaction removed from app mempool due to lower nonce being invalidated"
                    .into(),
                ..response::CheckTx::default()
            },
            RemovalReason::Included(block_number) => response::CheckTx {
                code: Code::Err(AbciErrorCode::TRANSACTION_PREVIOUSLY_INCLUDED.value()),
                info: AbciErrorCode::TRANSACTION_PREVIOUSLY_INCLUDED.to_string(),
                log: format!(
                    "transaction removed from app mempool because it was included in block \
                     {block_number}"
                ),
                ..response::CheckTx::default()
            },
        }
    }
}

impl IntoCheckTxResponse for InsertionError {
    fn into_check_tx_response(self) -> response::CheckTx {
        match self {
            InsertionError::AlreadyPresent => response::CheckTx {
                code: Code::Err(AbciErrorCode::ALREADY_PRESENT.value()),
                info: AbciErrorCode::ALREADY_PRESENT.to_string(),
                log: InsertionError::AlreadyPresent.to_string(),
                ..response::CheckTx::default()
            },
            InsertionError::NonceTooLow => response::CheckTx {
                code: Code::Err(AbciErrorCode::INVALID_NONCE.value()),
                info: AbciErrorCode::INVALID_NONCE.to_string(),
                log: InsertionError::NonceTooLow.to_string(),
                ..response::CheckTx::default()
            },
            InsertionError::NonceTaken => response::CheckTx {
                code: Code::Err(AbciErrorCode::NONCE_TAKEN.value()),
                info: AbciErrorCode::NONCE_TAKEN.to_string(),
                log: InsertionError::NonceTaken.to_string(),
                ..response::CheckTx::default()
            },
            InsertionError::AccountSizeLimit => response::CheckTx {
                code: Code::Err(AbciErrorCode::ACCOUNT_SIZE_LIMIT.value()),
                info: AbciErrorCode::ACCOUNT_SIZE_LIMIT.to_string(),
                log: InsertionError::AccountSizeLimit.to_string(),
                ..response::CheckTx::default()
            },
            InsertionError::ParkedSizeLimit => response::CheckTx {
                code: Code::Err(AbciErrorCode::PARKED_FULL.value()),
                info: AbciErrorCode::PARKED_FULL.info(),
                log: "transaction failed insertion because parked container is full".into(),
                ..response::CheckTx::default()
            },
            InsertionError::AccountBalanceTooLow | InsertionError::NonceGap => {
                // NOTE: these are handled interally by the mempool and don't
                // block transaction inclusion in the mempool. they shouldn't
                // be bubbled up to the client.
                response::CheckTx {
                    code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                    info: AbciErrorCode::INTERNAL_ERROR.info(),
                    log: "transaction failed insertion because of an internal error".into(),
                    ..response::CheckTx::default()
                }
            }
        }
    }
}

fn error_response(abci_error_code: AbciErrorCode, log: String) -> response::CheckTx {
    response::CheckTx {
        code: Code::Err(abci_error_code.value()),
        info: abci_error_code.info(),
        log,
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
/// # Errors
/// - if conversion of raw transaction bytes to a signed transaction fails
/// - if [`check_tx`] fails
#[instrument(skip_all)]
pub(crate) async fn handle_check_tx_request<S: StateRead>(
    req: request::CheckTx,
    state: S,
    mempool: &AppMempool,
    metrics: &'static Metrics,
) -> response::CheckTx {
    let request::CheckTx {
        tx, ..
    } = req;

    let signed_tx = match convert_bytes_to_signed_tx(tx, metrics).await {
        Ok(tx) => tx,
        Err(err) => return err.into(),
    };

    match check_tx(signed_tx.clone(), state, mempool, metrics).await {
        // Don't error if the transaction is already in the mempool
        Ok(()) | Err(CheckTxError::Tracked) => response::CheckTx::default(),
        Err(CheckTxError::RemovedFromMempool(reason)) => {
            // This statement should never fail, since we have a different `CheckTxError` variant
            // for included transactions.
            if !matches!(reason, RemovalReason::Included { .. }) {
                match reason {
                    RemovalReason::Expired => metrics.increment_check_tx_removed_expired(),
                    RemovalReason::FailedPrepareProposal(_) => {
                        metrics.increment_check_tx_removed_failed_execution();
                    }
                    _ => {}
                };
                mempool
                    .remove_from_removal_cache(signed_tx.id().get())
                    .await;
            }
            CheckTxError::RemovedFromMempool(reason).into()
        }
        Err(err) => err.into(),
    }
}

/// Performs `CheckTx` for a given [`Transaction`]. This consists of checking that
/// the transaction is not already in the app-side mempool or has been removed
/// from it, performing stateless checks, and inserting the transaction into the
/// mempool.
///
/// The function will return `Ok(())` if the transaction:
/// - Is already in the appside mempool
/// - Passes stateless checks and insertion into the mempool is successful
///
/// # Errors
/// - if the transaction has been removed from the app's mempool (will throw error once)
/// - if the transaction fails stateless checks
/// - if the transaction fails insertion into the mempool
pub(crate) async fn check_tx<S: StateRead>(
    tx: Transaction,
    state: S,
    mempool: &AppMempool,
    metrics: &'static Metrics,
) -> Result<(), CheckTxError> {
    let tx_hash = tx.id().get();

    let tx_status_start = Instant::now();
    let res = match mempool.transaction_status(&tx_hash).await {
        Some(TransactionStatus::Parked | TransactionStatus::Pending) => Err(CheckTxError::Tracked),
        Some(TransactionStatus::Included(block_number)) => Err(CheckTxError::Included {
            block_number,
        }),
        Some(TransactionStatus::Removed(reason)) => Err(CheckTxError::RemovedFromMempool(reason)),
        None => Ok(()),
    };
    metrics.record_check_tx_duration_seconds_transaction_status(tx_status_start.elapsed());
    res?;

    // perform stateless checks
    stateless_checks(tx.clone(), &state, metrics).await?;

    // attempt to insert the transaction into the mempool
    insert_into_mempool(mempool, &state, tx.clone(), metrics).await?;

    metrics.record_transaction_in_mempool_size_bytes(tx.to_raw().encoded_len());
    metrics.set_transactions_in_mempool_total(mempool.len().await);

    Ok(())
}

#[instrument(skip_all)]
async fn convert_bytes_to_signed_tx(
    tx: Bytes,
    metrics: &'static Metrics,
) -> Result<Transaction, CheckTxError> {
    let start_parsing = Instant::now();

    let tx_len = tx.len();

    if tx_len > MAX_TX_SIZE {
        metrics.increment_check_tx_removed_too_large();
        return Err(CheckTxError::TransactionTooLarge {
            max_size: MAX_TX_SIZE,
            actual_size: tx_len,
        });
    }

    let raw_signed_tx = match raw::Transaction::decode(tx) {
        Ok(tx) => tx,
        Err(err) => {
            return Err(CheckTxError::InvalidTransactionBytes {
                name: raw::Transaction::full_name(),
                source: err,
            });
        }
    };

    let signed_tx = match Transaction::try_from_raw(raw_signed_tx) {
        Ok(tx) => tx,
        Err(err) => {
            return Err(CheckTxError::InvalidTransactionProtobuf {
                source: err,
            });
        }
    };

    metrics.record_check_tx_duration_seconds_parse_tx(start_parsing.elapsed());

    Ok(signed_tx)
}

/// Performs stateless checks on the transaction.
///
/// Returns an `Err(response::CheckTx)` if the transaction fails any of the checks.
#[instrument(skip_all)]
async fn stateless_checks<S: StateRead>(
    signed_tx: Transaction,
    state: &S,
    metrics: &'static Metrics,
) -> Result<(), CheckTxError> {
    let start_check_stateless = Instant::now();

    if let Err(err) = signed_tx.check_stateless().await {
        metrics.increment_check_tx_removed_failed_stateless();
        return Err(CheckTxError::FailedStatelessChecks {
            source: err,
        });
    };

    let finished_check_stateless = Instant::now();
    metrics.record_check_tx_duration_seconds_check_stateless(
        finished_check_stateless.saturating_duration_since(start_check_stateless),
    );

    let expected_chain_id = state
        .get_chain_id()
        .await
        .map_err(|err| CheckTxError::InternalError {
            source: err,
        })?
        .to_string();
    if expected_chain_id != signed_tx.chain_id() {
        return Err(CheckTxError::InvalidChainId {
            expected: expected_chain_id.to_string(),
            actual: signed_tx.chain_id().to_string(),
        });
    }

    metrics.record_check_tx_duration_seconds_check_chain_id(finished_check_stateless.elapsed());

    Ok(())
}

/// Attempts to insert the transaction into the mempool.
///
/// Returns a `Err(response::CheckTx)` with an error code and message if the transaction fails
/// insertion into the mempool.
#[instrument(skip_all)]
async fn insert_into_mempool<S: StateRead>(
    mempool: &AppMempool,
    state: &S,
    signed_tx: Transaction,
    metrics: &'static Metrics,
) -> Result<(), CheckTxError> {
    let start_convert_address = Instant::now();

    // TODO: just use address bytes directly https://github.com/astriaorg/astria/issues/1620
    // generate address for the signed transaction
    let address = match state
        .try_base_prefixed(signed_tx.verification_key().address_bytes())
        .await
        .context("failed to generate address for signed transaction")
    {
        Err(err) => {
            return Err(CheckTxError::InternalError {
                source: err,
            });
        }
        Ok(address) => address,
    };

    let finished_convert_address = Instant::now();
    metrics.record_check_tx_duration_seconds_convert_address(
        finished_convert_address.saturating_duration_since(start_convert_address),
    );

    // fetch current account nonce
    let current_account_nonce = match state
        .get_account_nonce(&address)
        .await
        .wrap_err("failed fetching nonce for account")
    {
        Err(err) => {
            return Err(CheckTxError::InternalError {
                source: err,
            });
        }
        Ok(nonce) => nonce,
    };

    let finished_fetch_nonce = Instant::now();
    metrics.record_check_tx_duration_seconds_fetch_nonce(
        finished_fetch_nonce.saturating_duration_since(finished_convert_address),
    );

    // grab cost of transaction
    let transaction_cost = match transaction::get_total_transaction_cost(&signed_tx, &state)
        .await
        .context("failed fetching cost of the transaction")
    {
        Err(err) => {
            return Err(CheckTxError::InternalError {
                source: err,
            });
        }
        Ok(transaction_cost) => transaction_cost,
    };

    let finished_fetch_tx_cost = Instant::now();
    metrics.record_check_tx_duration_seconds_fetch_tx_cost(
        finished_fetch_tx_cost.saturating_duration_since(finished_fetch_nonce),
    );

    // grab current account's balances
    let current_account_balance: HashMap<IbcPrefixed, u128> =
        match get_account_balances(&state, &address)
            .await
            .with_context(|| "failed fetching balances for account `{address}`")
        {
            Err(err) => {
                return Err(CheckTxError::InternalError {
                    source: err,
                });
            }
            Ok(account_balance) => account_balance,
        };

    let finished_fetch_balances = Instant::now();
    metrics.record_check_tx_duration_seconds_fetch_balances(
        finished_fetch_balances.saturating_duration_since(finished_fetch_tx_cost),
    );

    let actions_count = signed_tx.actions().len();

    if let Err(err) = mempool
        .insert(
            Arc::new(signed_tx),
            current_account_nonce,
            &current_account_balance,
            transaction_cost,
        )
        .await
    {
        return Err(CheckTxError::FailedInsertion(err));
    }

    metrics
        .record_check_tx_duration_seconds_insert_to_app_mempool(finished_fetch_balances.elapsed());
    metrics.record_actions_per_transaction_in_mempool(actions_count);

    Ok(())
}
