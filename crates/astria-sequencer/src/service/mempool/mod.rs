#[cfg(test)]
mod tests;

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
    generated::protocol::transactions::v1alpha1 as raw,
    primitive::v1::asset::IbcPrefixed,
    protocol::{
        abci::AbciErrorCode,
        transaction::v1alpha1::SignedTransaction,
    },
};
use astria_eyre::eyre::WrapErr as _;
use bytes::Bytes;
use cnidarium::Storage;
use futures::{
    Future,
    FutureExt,
};
use prost::Message as _;
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
    accounts,
    address,
    app::ActionHandler as _,
    mempool::{
        get_account_balances,
        InsertionError,
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
/// This function will error if:
/// - the transaction has been removed from the app's mempool (will throw error once)
/// - the transaction fails stateless checks
/// - the transaction fails insertion into the mempool
///
/// The function will return a [`response::CheckTx`] with a status code of 0 if the transaction:
/// - Is already in the appside mempool
/// - Passes stateless checks and insertion into the mempool is successful
#[instrument(skip_all)]
async fn handle_check_tx<S: accounts::StateReadExt + address::StateReadExt + 'static>(
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
                return Err(response::CheckTx {
                    code: Code::Err(AbciErrorCode::TRANSACTION_EXPIRED.value()),
                    info: "transaction expired in app's mempool".into(),
                    log: "Transaction expired in the app's mempool".into(),
                    ..response::CheckTx::default()
                });
            }
            RemovalReason::FailedPrepareProposal(err) => {
                metrics.increment_check_tx_removed_failed_execution();
                return Err(response::CheckTx {
                    code: Code::Err(AbciErrorCode::TRANSACTION_FAILED.value()),
                    info: "transaction failed execution in prepare_proposal()".into(),
                    log: format!("transaction failed execution because: {err}"),
                    ..response::CheckTx::default()
                });
            }
            RemovalReason::NonceStale => {
                return Err(response::CheckTx {
                    code: Code::Err(AbciErrorCode::INVALID_NONCE.value()),
                    info: "transaction removed from app mempool due to stale nonce".into(),
                    log: "Transaction from app mempool due to stale nonce".into(),
                    ..response::CheckTx::default()
                });
            }
            RemovalReason::LowerNonceInvalidated => {
                return Err(response::CheckTx {
                    code: Code::Err(AbciErrorCode::LOWER_NONCE_INVALIDATED.value()),
                    info: "transaction removed from app mempool due to lower nonce being \
                           invalidated"
                        .into(),
                    log: "Transaction removed from app mempool due to lower nonce being \
                          invalidated"
                        .into(),
                    ..response::CheckTx::default()
                });
            }
        }
    };

    metrics.record_check_tx_duration_seconds_check_removed(start_removal_check.elapsed());

    Ok(())
}

/// Performs stateless checks on the transaction.
///
/// Returns an `Err(response::CheckTx)` if the transaction fails any of the checks.
/// Otherwise, it returns the [`SignedTransaction`] to be inserted into the mempool.
async fn stateless_checks<S: accounts::StateReadExt + address::StateReadExt + 'static>(
    tx: Bytes,
    state: &S,
    metrics: &'static Metrics,
) -> Result<SignedTransaction, response::CheckTx> {
    let start_parsing = Instant::now();

    let tx_len = tx.len();

    if tx_len > MAX_TX_SIZE {
        metrics.increment_check_tx_removed_too_large();
        return Err(response::CheckTx {
            code: Code::Err(AbciErrorCode::TRANSACTION_TOO_LARGE.value()),
            log: format!(
                "transaction size too large; allowed: {MAX_TX_SIZE} bytes, got {}",
                tx.len()
            ),
            info: AbciErrorCode::TRANSACTION_TOO_LARGE.info(),
            ..response::CheckTx::default()
        });
    }

    let raw_signed_tx = match raw::SignedTransaction::decode(tx) {
        Ok(tx) => tx,
        Err(e) => {
            return Err(response::CheckTx {
                code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
                log: format!("{e:#}"),
                info: "failed decoding bytes as a protobuf SignedTransaction".into(),
                ..response::CheckTx::default()
            });
        }
    };
    let signed_tx = match SignedTransaction::try_from_raw(raw_signed_tx) {
        Ok(tx) => tx,
        Err(e) => {
            return Err(response::CheckTx {
                code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
                info: "the provided bytes was not a valid protobuf-encoded SignedTransaction, or \
                       the signature was invalid"
                    .into(),
                log: format!("{e:#}"),
                ..response::CheckTx::default()
            });
        }
    };

    let finished_parsing = Instant::now();
    metrics.record_check_tx_duration_seconds_parse_tx(
        finished_parsing.saturating_duration_since(start_parsing),
    );

    if let Err(e) = signed_tx.check_stateless().await {
        metrics.increment_check_tx_removed_failed_stateless();
        return Err(response::CheckTx {
            code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
            info: "transaction failed stateless check".into(),
            log: format!("{e:#}"),
            ..response::CheckTx::default()
        });
    };

    let finished_check_stateless = Instant::now();
    metrics.record_check_tx_duration_seconds_check_stateless(
        finished_check_stateless.saturating_duration_since(finished_parsing),
    );

    if let Err(e) = transaction::check_chain_id_mempool(&signed_tx, &state).await {
        return Err(response::CheckTx {
            code: Code::Err(AbciErrorCode::INVALID_CHAIN_ID.value()),
            info: "failed verifying chain id".into(),
            log: format!("{e:#}"),
            ..response::CheckTx::default()
        });
    }

    metrics.record_check_tx_duration_seconds_check_chain_id(finished_check_stateless.elapsed());

    // note: decide if worth moving to post-insertion, would have to recalculate cost
    metrics.record_transaction_in_mempool_size_bytes(tx_len);

    Ok(signed_tx)
}

/// Attempts to insert the transaction into the mempool.
///
/// Returns a `Err(response::CheckTx)` with an error code and message if the transaction fails
/// insertion into the mempool.
#[expect(clippy::too_many_lines, reason = "error handling is long")]
async fn insert_into_mempool<S: accounts::StateReadExt + address::StateReadExt + 'static>(
    mempool: &AppMempool,
    state: &S,
    signed_tx: SignedTransaction,
    metrics: &'static Metrics,
) -> Result<(), response::CheckTx> {
    let start_convert_address = Instant::now();

    // generate address for the signed transaction
    let address = match state
        .try_base_prefixed(signed_tx.verification_key().address_bytes())
        .await
        .context("failed to generate address for signed transaction")
    {
        Err(err) => {
            return Err(response::CheckTx {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                log: format!("failed to generate address because: {err:#}"),
                ..response::CheckTx::default()
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
            return Err(response::CheckTx {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                log: format!("failed to fetch account nonce because: {err:#}"),
                ..response::CheckTx::default()
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
            return Err(response::CheckTx {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                log: format!("failed to fetch cost of the transaction because: {err:#}"),
                ..response::CheckTx::default()
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
                return Err(response::CheckTx {
                    code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                    info: AbciErrorCode::INTERNAL_ERROR.info(),
                    log: format!("failed to fetch account balances because: {err:#}"),
                    ..response::CheckTx::default()
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
            current_account_balance,
            transaction_cost,
        )
        .await
    {
        match err {
            InsertionError::NonceTooLow => {
                return Err(response::CheckTx {
                    code: Code::Err(AbciErrorCode::INVALID_NONCE.value()),
                    info: "transaction failed because account nonce is too low".into(),
                    log: format!("transaction failed because account nonce is too low: {err:#}"),
                    ..response::CheckTx::default()
                });
            }
            _ => {
                return Err(response::CheckTx {
                    code: Code::Err(AbciErrorCode::TRANSACTION_INSERTION_FAILED.value()),
                    info: "transaction insertion failed".into(),
                    log: format!("transaction insertion failed because: {err:#}"),
                    ..response::CheckTx::default()
                });
            }
        }
    }

    metrics
        .record_check_tx_duration_seconds_insert_to_app_mempool(finished_fetch_balances.elapsed());
    metrics.record_actions_per_transaction_in_mempool(actions_count);

    Ok(())
}
