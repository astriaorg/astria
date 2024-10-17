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
    generated::protocol::transaction::v1 as raw,
    primitive::v1::asset::IbcPrefixed,
    protocol::{
        abci::AbciErrorCode,
        transaction::v1::Transaction,
    },
    Protobuf as _,
};
use astria_eyre::eyre::WrapErr as _;
use bytes::Bytes;
use cnidarium::StateRead;
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
    address::StateReadExt as _,
    app::ActionHandler as _,
    mempool::{
        get_account_balances,
        InsertionError,
        Mempool as AppMempool,
        RemovalReason,
    },
    metrics::Metrics,
    storage::Storage,
    transaction,
};

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
/// - the transaction fails stateless checks
/// - the transaction fails insertion into the mempool
///
/// The function will return a [`response::CheckTx`] with a status code of 0 if the transaction:
/// - Is already in the appside mempool
/// - Passes stateless checks and insertion into the mempool is successful
#[instrument(skip_all)]
async fn handle_check_tx<S: StateRead>(
    req: request::CheckTx,
    state: S,
    mempool: &mut AppMempool,
    metrics: &'static Metrics,
) -> response::CheckTx {
    use sha2::Digest as _;

    let request::CheckTx {
        tx, ..
    } = req;

    let tx_hash = sha2::Sha256::digest(&tx).into();

    // check if the transaction has been removed from the appside mempool
    if let Err(rsp) = check_removed_comet_bft(tx_hash, mempool, metrics).await {
        return rsp;
    }

    // check if the transaction is already in the mempool
    if is_tracked(tx_hash, mempool, metrics).await {
        return response::CheckTx::default();
    }

    // perform stateless checks
    let signed_tx = match stateless_checks(tx, &state, metrics).await {
        Ok(signed_tx) => signed_tx,
        Err(rsp) => return rsp,
    };

    // attempt to insert the transaction into the mempool
    if let Err(rsp) = insert_into_mempool(mempool, &state, signed_tx, metrics).await {
        return rsp;
    }

    // insertion successful
    metrics.set_transactions_in_mempool_total(mempool.len().await);

    response::CheckTx::default()
}

/// Checks if the transaction is already in the mempool.
async fn is_tracked(tx_hash: [u8; 32], mempool: &AppMempool, metrics: &Metrics) -> bool {
    let start_tracked_check = Instant::now();

    let result = mempool.is_tracked(tx_hash).await;

    metrics.record_check_tx_duration_seconds_check_tracked(start_tracked_check.elapsed());

    result
}

/// Checks if the transaction has been removed from the appside mempool.
///
/// Returns an `Err(response::CheckTx)` with an error code and message if the transaction has been
/// removed from the appside mempool.
async fn check_removed_comet_bft(
    tx_hash: [u8; 32],
    mempool: &AppMempool,
    metrics: &Metrics,
) -> Result<(), response::CheckTx> {
    let start_removal_check = Instant::now();

    // check if the transaction has been removed from the appside mempool and handle
    // the removal reason
    if let Some(removal_reason) = mempool.check_removed_comet_bft(tx_hash).await {
        match removal_reason {
            RemovalReason::Expired => {
                metrics.increment_check_tx_removed_expired();
                return Err(removal_reason.into_check_tx_response());
            }
            RemovalReason::FailedPrepareProposal(_) => {
                metrics.increment_check_tx_removed_failed_execution();
                return Err(removal_reason.into_check_tx_response());
            }
            _ => return Err(removal_reason.into_check_tx_response()),
        }
    };

    metrics.record_check_tx_duration_seconds_check_removed(start_removal_check.elapsed());

    Ok(())
}

/// Performs stateless checks on the transaction.
///
/// Returns an `Err(response::CheckTx)` if the transaction fails any of the checks.
/// Otherwise, it returns the [`Transaction`] to be inserted into the mempool.
async fn stateless_checks<S: StateRead>(
    tx: Bytes,
    state: &S,
    metrics: &'static Metrics,
) -> Result<Transaction, response::CheckTx> {
    let start_parsing = Instant::now();

    let tx_len = tx.len();

    if tx_len > MAX_TX_SIZE {
        metrics.increment_check_tx_removed_too_large();
        return Err(error_response(
            AbciErrorCode::TRANSACTION_TOO_LARGE,
            format!(
                "transaction size too large; allowed: {MAX_TX_SIZE} bytes, got {}",
                tx.len()
            ),
        ));
    }

    let raw_signed_tx = match raw::Transaction::decode(tx) {
        Ok(tx) => tx,
        Err(e) => {
            return Err(error_response(
                AbciErrorCode::INVALID_PARAMETER,
                format!(
                    "failed decoding bytes as a protobuf {}: {e:#}",
                    raw::Transaction::full_name()
                ),
            ));
        }
    };
    let signed_tx = match Transaction::try_from_raw(raw_signed_tx) {
        Ok(tx) => tx,
        Err(e) => {
            return Err(error_response(
                AbciErrorCode::INVALID_PARAMETER,
                format!("the provided transaction could not be validated: {e:?}",),
            ));
        }
    };

    let finished_parsing = Instant::now();
    metrics.record_check_tx_duration_seconds_parse_tx(
        finished_parsing.saturating_duration_since(start_parsing),
    );

    if let Err(e) = signed_tx.check_stateless().await {
        metrics.increment_check_tx_removed_failed_stateless();
        return Err(error_response(
            AbciErrorCode::INVALID_PARAMETER,
            format!("transaction failed stateless check: {e:#}"),
        ));
    };

    let finished_check_stateless = Instant::now();
    metrics.record_check_tx_duration_seconds_check_stateless(
        finished_check_stateless.saturating_duration_since(finished_parsing),
    );

    if let Err(e) = transaction::check_chain_id_mempool(&signed_tx, &state).await {
        return Err(error_response(
            AbciErrorCode::INVALID_CHAIN_ID,
            format!("failed verifying chain id: {e:#}"),
        ));
    }

    metrics.record_check_tx_duration_seconds_check_chain_id(finished_check_stateless.elapsed());

    // NOTE: decide if worth moving to post-insertion, would have to recalculate cost
    metrics.record_transaction_in_mempool_size_bytes(tx_len);

    Ok(signed_tx)
}

/// Attempts to insert the transaction into the mempool.
///
/// Returns a `Err(response::CheckTx)` with an error code and message if the transaction fails
/// insertion into the mempool.
async fn insert_into_mempool<S: StateRead>(
    mempool: &AppMempool,
    state: &S,
    signed_tx: Transaction,
    metrics: &'static Metrics,
) -> Result<(), response::CheckTx> {
    let start_convert_address = Instant::now();

    // TODO: just use address bytes directly https://github.com/astriaorg/astria/issues/1620
    // generate address for the signed transaction
    let address = match state
        .try_base_prefixed(signed_tx.verification_key().address_bytes())
        .await
        .context("failed to generate address for signed transaction")
    {
        Err(err) => {
            return Err(error_response(
                AbciErrorCode::INTERNAL_ERROR,
                format!("failed to generate address because: {err:#}"),
            ));
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
            return Err(error_response(
                AbciErrorCode::INTERNAL_ERROR,
                format!("failed to fetch account nonce because: {err:#}"),
            ));
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
            return Err(error_response(
                AbciErrorCode::INTERNAL_ERROR,
                format!("failed to fetch cost of the transaction because: {err:#}"),
            ));
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
                return Err(error_response(
                    AbciErrorCode::INTERNAL_ERROR,
                    format!("failed to fetch account balances because: {err:#}"),
                ));
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
            current_account_balance,
            transaction_cost,
        )
        .await
    {
        return Err(err.into_check_tx_response());
    }

    metrics
        .record_check_tx_duration_seconds_insert_to_app_mempool(finished_fetch_balances.elapsed());
    metrics.record_actions_per_transaction_in_mempool(actions_count);

    Ok(())
}
