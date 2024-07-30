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
    generated::protocol::transaction::v1alpha1 as raw,
    protocol::{
        abci::AbciErrorCode,
        transaction::v1alpha1::SignedTransaction,
    },
};
use bytes::Bytes;
use cnidarium::Storage;
use futures::{
    Future,
    FutureExt,
};
use prost::Message as _;
use quick_cache::sync::Cache;
use sha2::Digest as _;
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
    warn,
    Instrument as _,
};

use crate::{
    accounts::state_ext::StateReadExt,
    mempool::{
        Mempool as AppMempool,
        RemovalReason,
    },
    metrics::Metrics,
    transaction,
};

const MAX_TX_SIZE: usize = 256_000; // 256 KB
/// The number of entries in the immutable checks cache.
const CACHE_SIZE: usize = 10_000;

type ImmutableChecksResult = Result<Arc<SignedTransaction>, response::CheckTx>;

/// `Mempool` handles [`request::CheckTx`] abci requests.
///
/// It performs stateless and stateful checks of the given transaction,
/// returning a [`response::CheckTx`].
#[derive(Clone)]
pub(crate) struct Mempool {
    storage: Storage,
    inner: AppMempool,
    /// A cache of recent results of immutable checks, indexed by tx hash.
    cached_immutable_checks: Arc<Cache<[u8; 32], ImmutableChecksResult>>,
    metrics: &'static Metrics,
}

impl Mempool {
    pub(crate) fn new(storage: Storage, mempool: AppMempool, metrics: &'static Metrics) -> Self {
        Self {
            storage,
            inner: mempool,
            cached_immutable_checks: Arc::new(Cache::new(CACHE_SIZE)),
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
        let cached_immutable_checks = self.cached_immutable_checks.clone();
        let metrics = self.metrics;
        async move {
            let rsp = match req {
                MempoolRequest::CheckTx(req) => MempoolResponse::CheckTx(
                    handle_check_tx(
                        req,
                        storage.latest_snapshot(),
                        mempool,
                        cached_immutable_checks,
                        metrics,
                    )
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
#[instrument(skip_all)]
async fn handle_check_tx<S: StateReadExt + 'static>(
    request::CheckTx {
        tx,
        kind,
    }: request::CheckTx,
    state: S,
    mempool: AppMempool,
    cached_immutable_checks: Arc<Cache<[u8; 32], ImmutableChecksResult>>,
    metrics: &'static Metrics,
) -> response::CheckTx {
    let start = Instant::now();

    // So we don't waste time hashing a large object, we don't check the cache before the size
    // check.
    let tx_len = tx.len();
    if tx_len > MAX_TX_SIZE {
        metrics.increment_check_tx_removed_too_large();
        return new_error_response(
            AbciErrorCode::TRANSACTION_TOO_LARGE,
            format!(
                "transaction size too large; allowed: {MAX_TX_SIZE} bytes, got: {tx_len} bytes",
            ),
        );
    }

    // Ok to hash the tx now and check in the cache.
    let tx_hash = sha2::Sha256::digest(&tx).into();
    let signed_tx = match cached_immutable_checks
        .get_value_or_guard_async(&tx_hash)
        .await
    {
        Ok(Ok(cached_tx)) => {
            // The previous `parse_and_run_immutable_checks` call was `Ok`: rerun mutable checks.
            metrics.increment_check_tx_cache_hit();
            cached_tx
        }
        Ok(Err(cached_error_response)) => {
            // The previous `parse_and_run_immutable_checks` call was `Err`: just return it.
            metrics.increment_check_tx_cache_hit();
            return cached_error_response;
        }
        Err(guard) => {
            if kind == request::CheckTxKind::Recheck {
                warn!(
                    tx_hash = %telemetry::display::base64(&tx_hash),
                    "got a cache miss for recheck of tx"
                );
                metrics.increment_check_tx_cache_miss();
            }
            let immutable_checks_result =
                parse_tx_and_run_immutable_checks(tx, start, &state, metrics).await;

            if guard.insert(immutable_checks_result.clone()).is_err() {
                warn!(
                    tx_hash = %telemetry::display::base64(&tx_hash),
                    "failed to cache the check tx result"
                );
            }

            match immutable_checks_result {
                Ok(tx) => tx,
                Err(response) => return response,
            }
        }
    };

    run_mutable_checks(signed_tx, tx_hash, tx_len, state, mempool, metrics).await
}

/// Parses and returns the signed tx from the request if and only if it passes immutable checks,
/// i.e. checks which will always pass or always fail.
async fn parse_tx_and_run_immutable_checks<S: StateReadExt + 'static>(
    serialized_tx: Bytes,
    mut start: Instant,
    state: &S,
    metrics: &'static Metrics,
) -> ImmutableChecksResult {
    let raw_signed_tx = match raw::SignedTransaction::decode(serialized_tx) {
        Ok(tx) => tx,
        Err(e) => {
            return Err(new_error_response(
                AbciErrorCode::INVALID_PARAMETER,
                format!("failed decoding bytes as a protobuf SignedTransaction: {e}"),
            ));
        }
    };
    let signed_tx = match SignedTransaction::try_from_raw(raw_signed_tx) {
        Ok(tx) => tx,
        Err(e) => {
            return Err(new_error_response(
                AbciErrorCode::INVALID_PARAMETER,
                format!(
                    "the provided bytes were not a valid protobuf-encoded SignedTransaction, or \
                     the signature was invalid: {e:#}"
                ),
            ));
        }
    };

    let mut end = Instant::now();
    metrics.record_check_tx_duration_seconds_parse_tx(end.saturating_duration_since(start));
    start = end;

    if let Err(e) = transaction::check_stateless(&signed_tx).await {
        metrics.increment_check_tx_removed_failed_stateless();
        return Err(new_error_response(
            AbciErrorCode::INVALID_PARAMETER,
            format!("transaction failed stateless check: {e:#}"),
        ));
    };

    end = Instant::now();
    metrics.record_check_tx_duration_seconds_check_stateless(end.saturating_duration_since(start));
    start = end;

    if let Err(e) = transaction::check_chain_id_mempool(&signed_tx, state).await {
        return Err(new_error_response(
            AbciErrorCode::INVALID_CHAIN_ID,
            format!("{e:#}"),
        ));
    }

    metrics.record_check_tx_duration_seconds_check_chain_id(start.elapsed());

    Ok(Arc::new(signed_tx))
}

async fn run_mutable_checks<S: StateReadExt + 'static>(
    signed_tx: Arc<SignedTransaction>,
    tx_hash: [u8; 32],
    tx_len: usize,
    state: S,
    mempool: AppMempool,
    metrics: &'static Metrics,
) -> response::CheckTx {
    let mut start = Instant::now();
    let current_account_nonce =
        match transaction::get_current_nonce_if_tx_nonce_valid(&signed_tx, &state).await {
            Ok(nonce) => nonce,
            Err(e) => {
                mempool.remove(tx_hash).await;
                metrics.increment_check_tx_removed_stale_nonce();
                return new_error_response(AbciErrorCode::INVALID_NONCE, format!("{e:#}"));
            }
        };

    let mut end = Instant::now();
    metrics.record_check_tx_duration_seconds_check_nonce(end.saturating_duration_since(start));
    start = end;

    if let Err(e) = transaction::check_balance_mempool(&signed_tx, &state).await {
        mempool.remove(tx_hash).await;
        metrics.increment_check_tx_removed_account_balance();
        return new_error_response(AbciErrorCode::INSUFFICIENT_FUNDS, format!("{e:#}"));
    };

    end = Instant::now();
    metrics.record_check_tx_duration_seconds_check_balance(end.saturating_duration_since(start));
    start = end;

    if let Some(removal_reason) = mempool.check_removed_comet_bft(tx_hash).await {
        mempool.remove(tx_hash).await;

        match removal_reason {
            RemovalReason::Expired => {
                metrics.increment_check_tx_removed_expired();
                return new_error_response(
                    AbciErrorCode::TRANSACTION_EXPIRED,
                    "transaction expired in the app's mempool",
                );
            }
            RemovalReason::FailedPrepareProposal(err) => {
                metrics.increment_check_tx_removed_failed_execution();
                return new_error_response(
                    AbciErrorCode::TRANSACTION_FAILED,
                    format!("transaction failed execution: {err:#}"),
                );
            }
        }
    };

    end = Instant::now();
    metrics.record_check_tx_duration_seconds_check_removed(end.saturating_duration_since(start));
    start = end;

    // tx is valid, push to mempool
    let actions_count = signed_tx.actions().len();
    mempool
        .insert(signed_tx, current_account_nonce)
        .await
        .expect(
            "tx nonce is greater than or equal to current account nonce; this was checked in \
             check_nonce_mempool",
        );
    let mempool_len = mempool.len().await;

    metrics.record_check_tx_duration_seconds_insert_to_app_mempool(start.elapsed());
    metrics.record_actions_per_transaction_in_mempool(actions_count);
    metrics.record_transaction_in_mempool_size_bytes(tx_len);
    metrics.set_transactions_in_mempool_total(mempool_len);

    response::CheckTx::default()
}

fn new_error_response<T: AsRef<str>>(code: AbciErrorCode, log: T) -> response::CheckTx {
    response::CheckTx {
        code: tendermint::abci::Code::from(code),
        info: code.info().to_string(),
        log: log.as_ref().to_string(),
        ..response::CheckTx::default()
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        crypto::SigningKey,
        protocol::transaction::v1alpha1::{
            action::ValidatorUpdate,
            Action,
            TransactionParams,
            UnsignedTransaction,
        },
    };
    use cnidarium::{
        StateDelta,
        TempStorage,
    };
    use tendermint::abci::Code;

    use super::*;
    use crate::{
        accounts::state_ext::StateWriteExt as _,
        bridge::state_ext::StateWriteExt as _,
        ibc::state_ext::StateWriteExt as _,
        state_ext::StateWriteExt as _,
    };

    #[tokio::test]
    async fn should_cache_failure() {
        let storage = TempStorage::new().await.unwrap();
        let mempool = AppMempool::new();
        let cached_immutable_checks = Arc::new(Cache::new(CACHE_SIZE));
        let metrics = Box::leak(Box::new(Metrics::new()));
        let request = request::CheckTx {
            tx: Bytes::new(),
            kind: request::CheckTxKind::New,
        };
        let tx_hash: [u8; 32] = sha2::Sha256::digest(&request.tx).into();

        // Should fail to parse and get added to the cache as `Err(response::CheckTx)`.
        let response = handle_check_tx(
            request,
            storage.latest_snapshot(),
            mempool.clone(),
            cached_immutable_checks.clone(),
            metrics,
        )
        .await;
        assert_eq!(
            response.code,
            AbciErrorCode::INVALID_PARAMETER.into(),
            "{response:?}"
        );
        assert_eq!(cached_immutable_checks.len(), 1);
        let cached_result = cached_immutable_checks.get(&tx_hash).unwrap();
        assert_eq!(cached_result.unwrap_err(), response);
    }

    #[tokio::test]
    async fn should_cache_success() {
        let nonce = 1;
        let chain_id = "chain-id";

        let storage = TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_delta = StateDelta::new(snapshot);
        state_delta
            .put_chain_id_and_revision_number(tendermint::chain::Id::try_from(chain_id).unwrap());
        state_delta.put_transfer_base_fee(1).unwrap();
        state_delta.put_ics20_withdrawal_base_fee(1).unwrap();
        state_delta.put_init_bridge_account_base_fee(1);
        state_delta.put_bridge_lock_byte_cost_multiplier(1);
        state_delta.put_bridge_sudo_change_base_fee(1);
        let mempool = AppMempool::new();
        let cached_immutable_checks = Arc::new(Cache::new(CACHE_SIZE));
        let metrics = Box::leak(Box::new(Metrics::new()));
        let signing_key = SigningKey::from([1; 32]);
        let action = ValidatorUpdate {
            power: 0,
            verification_key: signing_key.verification_key(),
        };
        let unsigned_tx = UnsignedTransaction {
            actions: vec![Action::ValidatorUpdate(action)],
            params: TransactionParams::builder()
                .nonce(nonce)
                .chain_id(chain_id)
                .build(),
        };
        let signed_tx = unsigned_tx.into_signed(&signing_key).to_raw();
        let request = request::CheckTx {
            tx: signed_tx.encode_to_vec().into(),
            kind: request::CheckTxKind::New,
        };
        let tx_hash: [u8; 32] = sha2::Sha256::digest(&request.tx).into();

        // Should parse, pass immutable checks and get added to the cache as
        // `Ok(SignedTransaction)`.
        let response = handle_check_tx(
            request,
            state_delta,
            mempool.clone(),
            cached_immutable_checks.clone(),
            metrics,
        )
        .await;
        assert_eq!(response.code, Code::Ok, "{response:?}");
        assert_eq!(cached_immutable_checks.len(), 1);
        let cached_result = cached_immutable_checks.get(&tx_hash).unwrap();
        assert_eq!(cached_result.unwrap().to_raw(), signed_tx);
    }
}
