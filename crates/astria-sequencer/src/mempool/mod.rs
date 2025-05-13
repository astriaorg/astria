#[cfg(feature = "benchmark")]
mod benchmarks;
use futures::TryStreamExt;
mod mempool_state;
mod recent_execution_results;
mod transactions_container;

use std::{
    collections::{
        HashMap,
        HashSet,
        VecDeque,
    },
    num::NonZeroUsize,
    sync::Arc,
};

use astria_core::{
    crypto::ADDRESS_LENGTH,
    primitive::v1::{
        asset::IbcPrefixed,
        TransactionId,
    },
};
use astria_eyre::eyre::Result;
use cnidarium::Snapshot;
use futures::TryFutureExt as _;
pub(crate) use mempool_state::get_account_balances;
use recent_execution_results::RecentExecutionResults;
use tendermint::abci::types::ExecTxResult;
use tokio::{
    sync::RwLock,
    time::Duration,
    try_join,
};
use tracing::{
    error,
    instrument,
    warn,
    Level,
};
pub(crate) use transactions_container::InsertionError;
use transactions_container::{
    ParkedTransactions,
    PendingTransactions,
    TimemarkedTransaction,
    TransactionsContainer as _,
};

use crate::{
    accounts::{
        AddressBytes as _,
        AssetBalance,
        StateReadExt as _,
    },
    app::StateReadExt,
    checked_transaction::CheckedTransaction,
    Metrics,
};

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum RemovalReason {
    Expired,
    NonceStale,
    LowerNonceInvalidated,
    FailedPrepareProposal(String),
    InternalError,
    IncludedInBlock {
        height: u64,
        result: Arc<ExecTxResult>,
    },
}

impl std::fmt::Display for RemovalReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemovalReason::Expired => write!(f, "expired"),
            RemovalReason::NonceStale => write!(f, "stale nonce"),
            RemovalReason::LowerNonceInvalidated => write!(f, "lower nonce invalidated"),
            RemovalReason::FailedPrepareProposal(reason) => {
                write!(f, "failed execution: {reason}")
            }
            RemovalReason::InternalError => {
                write!(f, "internal mempool error")
            }
            RemovalReason::IncludedInBlock {
                height,
                result,
            } => {
                let json_result = serde_json::to_string(result)
                    .unwrap_or_else(|_| "failed to serialize result".to_string());
                write!(
                    f,
                    "included in sequencer block {height} with result: {json_result}"
                )
            }
        }
    }
}

/// How long transactions are considered valid in the mempool.
const TX_TTL: Duration = Duration::from_secs(240);
/// Max number of parked transactions allowed per account.
const MAX_PARKED_TXS_PER_ACCOUNT: usize = 15;
/// Max number of transactions to keep in the removal cache. Should be larger than the max number of
/// transactions allowed in the cometBFT mempool.
const REMOVAL_CACHE_SIZE: usize = 50_000;

/// `RemovalCache` is used to signal to `CometBFT` that a
/// transaction can be removed from the `CometBFT` mempool.
///
/// This is useful for when a transaction fails execution or when
/// a transaction is invalidated due to mempool removal policies.
#[cfg_attr(feature = "benchmark", derive(Clone))]
struct RemovalCache {
    cache: HashMap<TransactionId, RemovalReason>,
    remove_queue: VecDeque<TransactionId>,
    max_size: NonZeroUsize,
}

impl RemovalCache {
    fn new(max_size: NonZeroUsize) -> Self {
        Self {
            cache: HashMap::new(),
            remove_queue: VecDeque::with_capacity(max_size.into()),
            max_size,
        }
    }

    /// Returns Some(RemovalReason) if the transaction is cached and
    /// removes the entry from the cache if present.
    fn remove(&mut self, tx_id: &TransactionId) -> Option<RemovalReason> {
        self.cache.remove(tx_id)
    }

    /// Adds the transaction to the cache, will preserve the original
    /// `RemovalReason` if already in the cache.
    fn add(&mut self, tx_id: TransactionId, reason: RemovalReason) {
        if self.cache.contains_key(&tx_id) {
            return;
        };

        if self.remove_queue.len() == usize::from(self.max_size) {
            // This should not happen if `REMOVAL_CACHE_SIZE` is >= CometBFT's configured mempool
            // size.
            //
            // Make space for the new transaction by removing the oldest transaction.
            let removed_tx_id = self
                .remove_queue
                .pop_front()
                .expect("cache should contain elements");
            warn!(
                %removed_tx_id,
                removal_cache_size = REMOVAL_CACHE_SIZE,
                "popped transaction from appside mempool removal cache, CometBFT will not remove \
                this transaction from its mempool - removal cache size possibly too low"
            );
            // Remove transaction from cache if it is present.
            self.cache.remove(&removed_tx_id);
        }
        self.remove_queue.push_back(tx_id);
        self.cache.insert(tx_id, reason);
    }
}

/// [`Mempool`] is an account-based structure for maintaining transactions for execution which is
/// safe and cheap to clone.
///
/// The transactions are split between pending and parked, where pending transactions are ready for
/// execution and parked transactions could be executable in the future.
///
/// The mempool exposes the pending transactions through `builder_queue()`, which returns a copy of
/// all pending transactions sorted in the order in which they should be executed. The sort order
/// is firstly by the difference between the transaction nonce and the account's current nonce
/// (ascending), and then by time first seen (ascending).
///
/// The mempool implements the following policies:
/// 1. Nonce replacement is not allowed.
/// 2. Accounts cannot have more than `MAX_PARKED_TXS_PER_ACCOUNT` transactions in their parked
///    queues.
/// 3. There is no account limit on pending transactions.
/// 4. Transactions will expire and can be removed after `TX_TTL` time.
/// 5. If an account has a transaction removed for being invalid or expired, all transactions for
///    that account with a higher nonce will be removed as well. This is due to the fact that we do
///    not execute failing transactions, so a transaction 'failing' will mean that further account
///    nonces will not be able to execute either.
///
/// Future extensions to this mempool can include:
/// - maximum mempool size
/// - account balance aware pending queue
#[derive(Clone)]
pub(crate) struct Mempool {
    inner: Arc<RwLock<MempoolInner>>,
}

impl Mempool {
    #[must_use]
    pub(crate) fn new(
        latest_snapshot: Snapshot,
        metrics: &'static Metrics,
        parked_max_tx_count: usize,
        execution_results_cache_size: usize,
    ) -> Self {
        Self {
            inner: Arc::new(RwLock::new(MempoolInner::new(
                latest_snapshot,
                metrics,
                parked_max_tx_count,
                execution_results_cache_size,
            ))),
        }
    }

    /// Returns the number of transactions in the mempool.
    #[must_use]
    #[instrument(skip_all)]
    pub(crate) async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// Inserts a transaction into the mempool and does not allow for transaction replacement.
    /// Will return the reason for insertion failure if failure occurs.
    #[instrument(skip_all, fields(tx_id = %checked_tx.id()), err(level = Level::DEBUG))]
    pub(crate) async fn insert(
        &self,
        checked_tx: Arc<CheckedTransaction>,
    ) -> Result<InsertionStatus, InsertionError> {
        self.inner.write().await.insert(checked_tx).await
    }

    /// Returns a copy of all transactions ready for execution, sorted first by the difference
    /// between a transaction and the account's current nonce and then by the time that the
    /// transaction was first seen by the appside mempool.
    #[instrument(skip_all)]
    pub(crate) async fn builder_queue(&self) -> Vec<Arc<CheckedTransaction>> {
        self.inner.read().await.builder_queue()
    }

    /// Removes the target transaction and all transactions for associated account with higher
    /// nonces.
    ///
    /// This function should only be used to remove invalid/failing transactions and not executed
    /// transactions. Executed transactions will be removed in the `run_maintenance()` function.
    #[instrument(skip_all, fields(tx_id = %checked_tx.id()))]
    pub(crate) async fn remove_tx_invalid(
        &self,
        checked_tx: Arc<CheckedTransaction>,
        reason: RemovalReason,
    ) {
        self.inner
            .write()
            .await
            .remove_tx_invalid(checked_tx, reason);
    }

    /// Removes the transaction from the CometBFT removal cache if it is present.
    #[instrument(skip_all, fields(tx_id))]
    pub(crate) async fn remove_from_removal_cache(&self, tx_id: &TransactionId) {
        self.inner.write().await.remove_from_removal_cache(tx_id);
    }

    /// Updates stored transactions to reflect current blockchain state. Will remove transactions
    /// that have stale nonces or are expired. Will also shift transaction between pending and
    /// parked to reflect changes in account balances.
    ///
    /// All removed transactions are added to the CometBFT removal cache to aid with CometBFT
    /// mempool maintenance.
    #[instrument(skip_all)]
    pub(crate) async fn run_maintenance(
        &self,
        latest_snapshot: Snapshot,
        recost: bool,
        block_execution_results: HashMap<TransactionId, Arc<ExecTxResult>>,
    ) {
        self.inner
            .write()
            .await
            .run_maintenance(latest_snapshot, recost, block_execution_results)
            .await;
    }

    /// Returns the highest pending nonce for the given address if it exists in the mempool. Note:
    /// does not take into account gapped nonces in the parked queue. For example, if the
    /// pending queue for an account has nonces [0,1] and the parked queue has [3], [1] will be
    /// returned.
    #[instrument(skip_all, fields(address = %telemetry::display::base64(address_bytes)))]
    pub(crate) async fn pending_nonce(&self, address_bytes: &[u8; ADDRESS_LENGTH]) -> Option<u32> {
        self.inner.read().await.pending_nonce(address_bytes)
    }

    #[instrument(skip_all)]
    pub(crate) async fn transaction_status(
        &self,
        tx_id: &TransactionId,
    ) -> Option<TransactionStatus> {
        self.inner.read().await.transaction_status(tx_id)
    }

    #[cfg(test)]
    pub(crate) async fn removal_cache(&self) -> HashMap<TransactionId, RemovalReason> {
        self.inner.read().await.removal_cache()
    }

    #[cfg(test)]
    pub(crate) async fn is_tracked(&self, tx_id: &TransactionId) -> bool {
        self.inner.read().await.is_tracked(tx_id)
    }

    #[cfg(feature = "benchmark")]
    pub(crate) async fn deep_clone(&self) -> Self {
        Self {
            inner: Arc::new(RwLock::new(self.inner.read().await.clone())),
        }
    }
}

#[derive(Debug)]
pub(crate) enum InsertionStatus {
    AddedToParked,
    AddedToPending,
}

#[cfg_attr(feature = "benchmark", derive(Clone))]
struct MempoolInner {
    pending: PendingTransactions,
    parked: ParkedTransactions<MAX_PARKED_TXS_PER_ACCOUNT>,
    comet_bft_removal_cache: RemovalCache,
    recent_execution_results: RecentExecutionResults,
    contained_txs: HashSet<TransactionId>,
    latest_snapshot: Snapshot,
    metrics: &'static Metrics,
}

impl MempoolInner {
    #[must_use]
    fn new(
        latest_snapshot: Snapshot,
        metrics: &'static Metrics,
        parked_max_tx_count: usize,
        execution_results_cache_size: usize,
    ) -> Self {
        Self {
            pending: PendingTransactions::new(TX_TTL),
            parked: ParkedTransactions::new(TX_TTL, parked_max_tx_count),
            comet_bft_removal_cache: RemovalCache::new(
                NonZeroUsize::try_from(REMOVAL_CACHE_SIZE)
                    .expect("Removal cache cannot be zero sized"),
            ),
            recent_execution_results: RecentExecutionResults::new(execution_results_cache_size),
            contained_txs: HashSet::new(),
            latest_snapshot,
            metrics,
        }
    }

    #[must_use]
    fn len(&self) -> usize {
        self.contained_txs.len()
    }

    async fn insert(
        &mut self,
        checked_tx: Arc<CheckedTransaction>,
    ) -> Result<InsertionStatus, InsertionError> {
        let address_bytes = *checked_tx.address_bytes();
        let current_account_nonce_fut = self
            .latest_snapshot
            .get_account_nonce(&address_bytes)
            .map_err(|error| InsertionError::Internal(error.to_string()));
        let current_account_balances_fut = self
            .latest_snapshot
            .account_asset_balances(&address_bytes)
            .map_ok(
                |AssetBalance {
                     asset,
                     balance,
                 }| (asset, balance),
            )
            // note: this relies on the IBC prefixed assets coming out of the stream to be unique
            .try_collect::<HashMap<IbcPrefixed, u128>>()
            .map_err(|error| InsertionError::Internal(error.to_string()));
        let tx_costs_fut = checked_tx
            .total_costs(&self.latest_snapshot)
            .map_err(InsertionError::FailedToCalculateCosts);

        let (current_account_nonce, current_account_balances, tx_costs) = try_join!(
            current_account_nonce_fut,
            current_account_balances_fut,
            tx_costs_fut
        )?;

        let ttx_to_insert = TimemarkedTransaction::new(checked_tx, tx_costs);
        let tx_id_to_insert = *ttx_to_insert.id();

        // try insert into pending
        match self.pending.add(
            ttx_to_insert.clone(),
            current_account_nonce,
            &current_account_balances,
        ) {
            Err(InsertionError::NonceGap | InsertionError::AccountBalanceTooLow) => {
                // try to add to parked queue
                match self.parked.add(
                    ttx_to_insert,
                    current_account_nonce,
                    &current_account_balances,
                ) {
                    Ok(()) => {
                        // log current size of parked
                        self.metrics
                            .set_transactions_in_mempool_parked(self.parked.len());

                        // track in contained txs
                        self.contained_txs.insert(tx_id_to_insert);
                        Ok(InsertionStatus::AddedToParked)
                    }
                    Err(err) => Err(err),
                }
            }
            Err(error) => Err(error),
            Ok(()) => {
                // check parked for txs able to be promoted
                let address_bytes = ttx_to_insert.address_bytes();
                let target_nonce = ttx_to_insert
                    .nonce()
                    .checked_add(1)
                    .expect("failed to increment nonce in promotion");
                let available_balances = self
                    .pending
                    .subtract_contained_costs(address_bytes, current_account_balances.clone());
                let promotables =
                    self.parked
                        .find_promotables(address_bytes, target_nonce, &available_balances);
                // promote the transactions
                for ttx_to_promote in promotables {
                    let tx_id_to_promote = *ttx_to_promote.id();
                    if let Err(error) = self.pending.add(
                        ttx_to_promote,
                        current_account_nonce,
                        &current_account_balances,
                    ) {
                        self.contained_txs.remove(&tx_id_to_promote);
                        self.comet_bft_removal_cache
                            .add(tx_id_to_promote, RemovalReason::InternalError);
                        error!(
                            current_account_nonce,
                            %tx_id_to_promote,
                            %error,
                            "failed to promote transaction during insertion"
                        );
                    }
                }

                // track in contained txs
                self.contained_txs.insert(tx_id_to_insert);

                Ok(InsertionStatus::AddedToPending)
            }
        }
    }

    fn builder_queue(&self) -> Vec<Arc<CheckedTransaction>> {
        self.pending.builder_queue()
    }

    fn remove_tx_invalid(&mut self, checked_tx: Arc<CheckedTransaction>, reason: RemovalReason) {
        let tx_id = *checked_tx.id();
        let address_bytes = *checked_tx.address_bytes();

        // Try to remove from pending.
        let removed_tx_ids = match self.pending.remove(checked_tx) {
            Ok(mut removed_tx_ids) => {
                // Remove all of parked.
                removed_tx_ids.append(&mut self.parked.clear_account(&address_bytes));
                removed_tx_ids
            }
            Err(checked_tx) => {
                // Not found in pending, try to remove from parked and if not found, just return.
                match self.parked.remove(checked_tx) {
                    Ok(removed_tx_ids) => removed_tx_ids,
                    Err(_) => return,
                }
            }
        };

        // Add the original tx first to preserve its reason for removal. The second
        // attempt to add it inside the loop below will be a no-op.
        self.comet_bft_removal_cache.add(tx_id, reason);
        for removed_tx_id in removed_tx_ids {
            self.contained_txs.remove(&removed_tx_id);
            self.comet_bft_removal_cache
                .add(removed_tx_id, RemovalReason::LowerNonceInvalidated);
        }
    }

    fn remove_from_removal_cache(&mut self, tx_id: &TransactionId) {
        self.comet_bft_removal_cache.remove(tx_id);
    }

    #[expect(clippy::too_many_lines, reason = "should be refactored")]
    async fn run_maintenance(
        &mut self,
        latest_snapshot: Snapshot,
        recost: bool,
        block_execution_results: HashMap<TransactionId, Arc<ExecTxResult>>,
    ) {
        self.latest_snapshot = latest_snapshot;
        let block_height = self
            .latest_snapshot
            .get_block_height()
            .await
            .unwrap_or_else(|error| {
                error!("failed to fetch block height while running mempool maintenance: {error:#}");
                0
            });
        let mut removed_txs = Vec::<(TransactionId, RemovalReason)>::new();

        // To clean we need to:
        // 1.) remove stale and expired transactions
        // 2.) recost remaining transactions if needed
        // 3.) check if we have transactions in pending which need to be demoted due
        //     to balance decreases
        // 4.) if there were no demotions, check if parked has transactions we can
        //     promote

        let addresses: HashSet<[u8; ADDRESS_LENGTH]> = self
            .pending
            .addresses()
            .chain(self.parked.addresses())
            .copied()
            .collect();

        // TODO: Make this concurrent, all account state is separate with IO bound disk reads.
        for address_bytes in &addresses {
            // get current account state
            let current_nonce = match self.latest_snapshot.get_account_nonce(address_bytes).await {
                Ok(res) => res,
                Err(error) => {
                    error!(
                        address = %telemetry::display::base64(&address_bytes),
                        "failed to fetch account nonce while running mempool maintenance: {error:#}"
                    );
                    continue;
                }
            };
            let current_balances = match get_account_balances(&self.latest_snapshot, address_bytes)
                .await
            {
                Ok(res) => res,
                Err(error) => {
                    error!(
                        address = %telemetry::display::base64(address_bytes),
                        "failed to fetch account balances while running mempool maintenance: {error:#}"
                    );
                    continue;
                }
            };

            // clean pending and parked of stale and expired
            removed_txs.extend(self.pending.clean_account_stale_expired(
                address_bytes,
                current_nonce,
                &block_execution_results,
                block_height,
            ));
            if recost {
                self.pending
                    .recost_transactions(address_bytes, &self.latest_snapshot)
                    .await;
            }

            removed_txs.extend(self.parked.clean_account_stale_expired(
                address_bytes,
                current_nonce,
                &block_execution_results,
                block_height,
            ));
            if recost {
                self.parked
                    .recost_transactions(address_bytes, &self.latest_snapshot)
                    .await;
            }

            // get transactions to demote from pending
            let demotion_txs = self
                .pending
                .find_demotables(address_bytes, &current_balances);

            if demotion_txs.is_empty() {
                // nothing to demote, check for transactions to promote
                let pending_nonce = self
                    .pending
                    .pending_nonce(address_bytes)
                    .map_or(current_nonce, |nonce| nonce);

                let remaining_balances = self
                    .pending
                    .subtract_contained_costs(address_bytes, current_balances.clone());
                let promotion_txs =
                    self.parked
                        .find_promotables(address_bytes, pending_nonce, &remaining_balances);

                for promotion_tx in promotion_txs {
                    let tx_id = *promotion_tx.id();
                    if let Err(error) =
                        self.pending
                            .add(promotion_tx, current_nonce, &current_balances)
                    {
                        self.contained_txs.remove(&tx_id);
                        self.metrics.increment_internal_logic_error();
                        error!(
                            address = %telemetry::display::base64(&address_bytes),
                            current_nonce, %tx_id, %error,
                            "failed to promote transaction during maintenance"
                        );
                    }
                }
            } else {
                // add demoted transactions to parked
                for demotion_tx in demotion_txs {
                    let tx_id = *demotion_tx.id();
                    if let Err(error) =
                        self.parked
                            .add(demotion_tx, current_nonce, &current_balances)
                    {
                        self.contained_txs.remove(&tx_id);
                        self.metrics.increment_internal_logic_error();
                        error!(
                            address = %telemetry::display::base64(&address_bytes),
                            current_nonce, %tx_id, %error,
                            "failed to demote transaction during maintenance"
                        );
                    }
                }
            }
        }

        // add to removal cache for cometbft and remove from the tracked set
        for (tx_id, reason) in removed_txs {
            self.contained_txs.remove(&tx_id);
            self.comet_bft_removal_cache.add(tx_id, reason);
        }
        self.recent_execution_results
            .add(block_execution_results, block_height);
        self.metrics
            .set_results_in_recently_executed_cache(self.recent_execution_results.len());
    }

    fn pending_nonce(&self, address_bytes: &[u8; ADDRESS_LENGTH]) -> Option<u32> {
        self.pending.pending_nonce(address_bytes)
    }

    fn transaction_status(&self, tx_id: &TransactionId) -> Option<TransactionStatus> {
        if self.contained_txs.contains(tx_id) {
            if self.pending.contains_tx(tx_id) {
                Some(TransactionStatus::Pending)
            } else {
                Some(TransactionStatus::Parked)
            }
        } else {
            self.recent_execution_results
                .get(tx_id)
                .map(|tx_data| {
                    TransactionStatus::Removed(RemovalReason::IncludedInBlock {
                        height: tx_data.block_height(),
                        result: tx_data.result(),
                    })
                })
                .or_else(|| {
                    self.comet_bft_removal_cache
                        .cache
                        .get(tx_id)
                        .map(|reason| TransactionStatus::Removed(reason.clone()))
                })
        }
    }

    #[cfg(test)]
    fn removal_cache(&self) -> HashMap<TransactionId, RemovalReason> {
        self.comet_bft_removal_cache.cache.clone()
    }

    #[cfg(test)]
    fn is_tracked(&self, tx_id: &TransactionId) -> bool {
        self.contained_txs.contains(tx_id)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum TransactionStatus {
    Pending,
    Parked,
    Removed(RemovalReason),
}

#[cfg(test)]
mod tests {
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        accounts::StateWriteExt as _,
        app::StateWriteExt as _,
        test_utils::{
            Fixture,
            ALICE,
            ALICE_ADDRESS_BYTES,
            BOB,
            BOB_ADDRESS_BYTES,
            CAROL_ADDRESS_BYTES,
        },
    };

    async fn put_alice_nonce(fixture: &Fixture, nonce: u32) {
        let storage = fixture.storage();
        let mut state_delta = StateDelta::new(storage.latest_snapshot());
        state_delta
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, nonce)
            .unwrap();
        let _ = storage.commit(state_delta).await.unwrap();
        fixture.mempool().inner.write().await.latest_snapshot = storage.latest_snapshot();
    }

    async fn put_block_height(fixture: &Fixture, height: u64) {
        let storage = fixture.storage();
        let mut state_delta = StateDelta::new(storage.latest_snapshot());
        state_delta.put_block_height(height).unwrap();
        let _ = storage.commit(state_delta).await.unwrap();
        fixture.mempool().inner.write().await.latest_snapshot = storage.latest_snapshot();
    }

    async fn new_alice_tx(fixture: &Fixture, nonce: u32) -> Arc<CheckedTransaction> {
        fixture
            .checked_tx_builder()
            .with_signer(ALICE.clone())
            .with_nonce(nonce)
            .build()
            .await
    }

    /// Sets Alice's balances to exactly match the total costs of the given tx.
    async fn set_alice_balances_to_cover_costs(fixture: &Fixture, tx: &Arc<CheckedTransaction>) {
        let costs = tx.total_costs(fixture.state()).await.unwrap();

        let storage = fixture.storage();
        let mut state_delta = StateDelta::new(storage.latest_snapshot());

        for (denom, cost) in costs {
            state_delta
                .put_account_balance(&*ALICE_ADDRESS_BYTES, &denom, cost)
                .unwrap();
        }

        let _ = storage.commit(state_delta).await.unwrap();
        fixture.mempool().inner.write().await.latest_snapshot = storage.latest_snapshot();
    }

    /// Increases Alice's balances by the total costs of the given tx.
    async fn increase_alice_balances_to_cover_costs(
        fixture: &Fixture,
        tx: &Arc<CheckedTransaction>,
    ) {
        let costs = tx.total_costs(fixture.state()).await.unwrap();

        let storage = fixture.storage();
        let mut state_delta = StateDelta::new(storage.latest_snapshot());

        for (denom, cost) in costs {
            let current_balance = state_delta
                .get_account_balance(&*ALICE_ADDRESS_BYTES, &denom)
                .await
                .unwrap();
            let new_balance = current_balance.checked_add(cost).unwrap();
            state_delta
                .put_account_balance(&*ALICE_ADDRESS_BYTES, &denom, new_balance)
                .unwrap();
        }

        let _ = storage.commit(state_delta).await.unwrap();
        fixture.mempool().inner.write().await.latest_snapshot = storage.latest_snapshot();
    }

    async fn assert_tx_in_pending(mempool: &Mempool, tx_id: &TransactionId) {
        assert_eq!(
            mempool.transaction_status(tx_id).await.unwrap(),
            TransactionStatus::Pending
        );
    }

    async fn assert_tx_in_parked(mempool: &Mempool, tx_id: &TransactionId) {
        assert_eq!(
            mempool.transaction_status(tx_id).await.unwrap(),
            TransactionStatus::Parked
        );
    }

    async fn assert_tx_removed(
        mempool: &Mempool,
        tx_id: &TransactionId,
        expected_reason: &RemovalReason,
    ) {
        assert_eq!(
            mempool.transaction_status(tx_id).await.unwrap(),
            TransactionStatus::Removed(expected_reason.clone())
        );
    }

    #[tokio::test]
    async fn insert() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        // sign and insert nonce 1
        let tx1 = new_alice_tx(&fixture, 1).await;
        assert!(
            mempool.insert(tx1.clone()).await.is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );
        assert_eq!(mempool.len().await, 1);

        // try to insert again
        assert!(
            matches!(
                mempool.insert(tx1).await.unwrap_err(),
                InsertionError::AlreadyPresent
            ),
            "already present"
        );

        // try to replace nonce
        let tx1_replacement = fixture
            .checked_tx_builder()
            .with_nonce(1)
            .with_rollup_data_submission(vec![2, 3, 4])
            .with_signer(ALICE.clone())
            .build()
            .await;
        assert!(
            matches!(
                mempool.insert(tx1_replacement).await.unwrap_err(),
                InsertionError::NonceTaken
            ),
            "nonce replace not allowed"
        );

        // add too low nonce
        put_alice_nonce(&fixture, 1).await;
        let tx0 = new_alice_tx(&fixture, 0).await;
        assert!(
            matches!(
                mempool.insert(tx0).await.unwrap_err(),
                InsertionError::NonceTooLow
            ),
            "nonce too low"
        );
    }

    #[tokio::test]
    async fn single_account_flow_extensive() {
        // This test tries to hit the more complex edges of the mempool with a single account.
        // The test adds the nonces [1,2,0,4], creates a builder queue, and then cleans the pool to
        // nonce 4. This tests some of the odder edge cases that can be hit if a node goes offline
        // or fails to see some transactions that other nodes include into their proposed blocks.
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        // add nonces in odd order to trigger insertion promotion logic
        let tx1 = new_alice_tx(&fixture, 1).await;
        let tx2 = new_alice_tx(&fixture, 2).await;
        let tx0 = new_alice_tx(&fixture, 0).await;
        let tx4 = new_alice_tx(&fixture, 4).await;
        mempool.insert(tx1.clone()).await.unwrap();
        mempool.insert(tx2.clone()).await.unwrap();
        mempool.insert(tx0.clone()).await.unwrap();
        mempool.insert(tx4.clone()).await.unwrap();

        // assert size
        assert_eq!(mempool.len().await, 4);

        // grab building queue, should return transactions [0,1,2] since [4] is gapped
        let builder_queue = mempool.builder_queue().await;

        // see contains first three transactions that should be pending
        assert_eq!(builder_queue, vec![tx0, tx1, tx2]);

        // see mempool's transactions just cloned, not consumed
        assert_eq!(mempool.len().await, 4);

        // run maintenance with simulated nonce to remove the nonces 0,1,2 and promote 4 from parked
        // to pending
        put_alice_nonce(&fixture, 4).await;

        mempool
            .run_maintenance(fixture.storage().latest_snapshot(), false, HashMap::new())
            .await;

        // assert mempool at 1
        assert_eq!(mempool.len().await, 1);

        // see transaction [4] properly promoted
        let builder_queue = mempool.builder_queue().await;
        assert_eq!(builder_queue, vec![tx4]);
    }

    #[tokio::test]
    async fn run_maintenance_promotion() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        // create transaction setup to trigger promotions
        //
        // initially pending has single transaction
        put_alice_nonce(&fixture, 1).await;
        let tx1 = new_alice_tx(&fixture, 1).await;
        set_alice_balances_to_cover_costs(&fixture, &tx1).await;
        let tx2 = new_alice_tx(&fixture, 2).await;
        let tx3 = new_alice_tx(&fixture, 3).await;
        let tx4 = new_alice_tx(&fixture, 4).await;

        mempool.insert(tx1.clone()).await.unwrap();
        mempool.insert(tx2.clone()).await.unwrap();
        mempool.insert(tx3.clone()).await.unwrap();
        mempool.insert(tx4.clone()).await.unwrap();

        // see pending only has one transaction
        let builder_queue = mempool.builder_queue().await;
        assert_eq!(builder_queue, vec![tx1.clone()]);

        // run maintenance with account containing balance for two more transactions
        increase_alice_balances_to_cover_costs(&fixture, &tx2).await;
        increase_alice_balances_to_cover_costs(&fixture, &tx3).await;

        mempool
            .run_maintenance(fixture.storage().latest_snapshot(), false, HashMap::new())
            .await;

        // see builder queue now contains them
        let builder_queue = mempool.builder_queue().await;
        assert_eq!(builder_queue, vec![tx1, tx2, tx3]);
    }

    #[tokio::test]
    async fn run_maintenance_demotion() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        // create transaction setup to trigger demotions
        //
        // initially pending has four transactions
        put_alice_nonce(&fixture, 1).await;
        let tx1 = new_alice_tx(&fixture, 1).await;
        let tx2 = new_alice_tx(&fixture, 2).await;
        let tx3 = new_alice_tx(&fixture, 3).await;
        let tx4 = new_alice_tx(&fixture, 4).await;

        mempool.insert(tx1.clone()).await.unwrap();
        mempool.insert(tx2.clone()).await.unwrap();
        mempool.insert(tx3.clone()).await.unwrap();
        mempool.insert(tx4.clone()).await.unwrap();

        // see pending only has all transactions
        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue,
            vec![tx1.clone(), tx2.clone(), tx3.clone(), tx4.clone()]
        );

        // setup state so Alice can only pay for first tx
        set_alice_balances_to_cover_costs(&fixture, &tx1).await;
        mempool
            .run_maintenance(fixture.storage().latest_snapshot(), false, HashMap::new())
            .await;

        // see builder queue now contains single transactions
        let builder_queue = mempool.builder_queue().await;
        assert_eq!(builder_queue, vec![tx1.clone()]);

        // setup state so Alice can now pay for first three txs
        increase_alice_balances_to_cover_costs(&fixture, &tx2).await;
        increase_alice_balances_to_cover_costs(&fixture, &tx3).await;

        mempool
            .run_maintenance(fixture.storage().latest_snapshot(), false, HashMap::new())
            .await;

        let builder_queue = mempool.builder_queue().await;
        assert_eq!(builder_queue, vec![tx1, tx2, tx3]);
    }

    #[tokio::test]
    async fn remove_invalid() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        // sign and insert nonces 0,1 and 3,4,5
        let tx0 = new_alice_tx(&fixture, 0).await;
        let tx1 = new_alice_tx(&fixture, 1).await;
        let tx3 = new_alice_tx(&fixture, 3).await;
        let tx4 = new_alice_tx(&fixture, 4).await;
        let tx5 = new_alice_tx(&fixture, 5).await;
        mempool.insert(tx0.clone()).await.unwrap();
        mempool.insert(tx1.clone()).await.unwrap();
        mempool.insert(tx3.clone()).await.unwrap();
        mempool.insert(tx4.clone()).await.unwrap();
        mempool.insert(tx5.clone()).await.unwrap();
        assert_tx_in_pending(&mempool, tx0.id()).await;
        assert_tx_in_pending(&mempool, tx1.id()).await;
        assert_tx_in_parked(&mempool, tx3.id()).await;
        assert_tx_in_parked(&mempool, tx4.id()).await;
        assert_tx_in_parked(&mempool, tx5.id()).await;

        let removal_reason = RemovalReason::FailedPrepareProposal("reason".to_string());

        // remove 4, should remove 4 and 5
        mempool
            .remove_tx_invalid(tx4.clone(), removal_reason.clone())
            .await;
        assert_eq!(mempool.len().await, 3);
        assert_tx_in_pending(&mempool, tx0.id()).await;
        assert_tx_in_pending(&mempool, tx1.id()).await;
        assert_tx_in_parked(&mempool, tx3.id()).await;
        assert_tx_removed(&mempool, tx4.id(), &removal_reason).await;
        assert_tx_removed(&mempool, tx5.id(), &RemovalReason::LowerNonceInvalidated).await;

        // remove 4 again is also ok
        mempool
            .remove_tx_invalid(
                tx4.clone(),
                RemovalReason::NonceStale, // shouldn't be inserted into removal cache
            )
            .await;
        assert_eq!(mempool.len().await, 3);
        assert_tx_in_pending(&mempool, tx0.id()).await;
        assert_tx_in_pending(&mempool, tx1.id()).await;
        assert_tx_in_parked(&mempool, tx3.id()).await;
        assert_tx_removed(&mempool, tx4.id(), &removal_reason).await;
        assert_tx_removed(&mempool, tx5.id(), &RemovalReason::LowerNonceInvalidated).await;

        // remove 1, should remove 1 and 3
        mempool
            .remove_tx_invalid(tx1.clone(), removal_reason.clone())
            .await;
        assert_eq!(mempool.len().await, 1);
        assert_tx_in_pending(&mempool, tx0.id()).await;
        assert_tx_removed(&mempool, tx1.id(), &removal_reason).await;
        assert_tx_removed(&mempool, tx3.id(), &RemovalReason::LowerNonceInvalidated).await;
        assert_tx_removed(&mempool, tx4.id(), &removal_reason).await;
        assert_tx_removed(&mempool, tx5.id(), &RemovalReason::LowerNonceInvalidated).await;

        // remove 0
        mempool
            .remove_tx_invalid(tx0.clone(), removal_reason.clone())
            .await;
        assert_eq!(mempool.len().await, 0);

        // assert that all were added to the cometbft removal cache
        // and the expected reasons were tracked
        assert_tx_removed(&mempool, tx0.id(), &removal_reason).await;
        assert_tx_removed(&mempool, tx1.id(), &removal_reason).await;
        assert_tx_removed(&mempool, tx3.id(), &RemovalReason::LowerNonceInvalidated).await;
        assert_tx_removed(&mempool, tx4.id(), &removal_reason).await;
        assert_tx_removed(&mempool, tx5.id(), &RemovalReason::LowerNonceInvalidated).await;
    }

    #[tokio::test]
    async fn should_get_pending_nonce() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        // sign and insert Alice txs with nonces 0,1
        let tx0 = new_alice_tx(&fixture, 0).await;
        let tx1 = new_alice_tx(&fixture, 1).await;
        mempool.insert(tx0.clone()).await.unwrap();
        mempool.insert(tx1.clone()).await.unwrap();

        // sign and insert Bob txs with nonces 100, 101
        let storage = fixture.storage();
        let mut state_delta = StateDelta::new(storage.latest_snapshot());
        state_delta
            .put_account_nonce(&*BOB_ADDRESS_BYTES, 100)
            .unwrap();
        let _ = storage.commit(state_delta).await.unwrap();
        fixture.mempool().inner.write().await.latest_snapshot = storage.latest_snapshot();
        let tx100 = fixture
            .checked_tx_builder()
            .with_nonce(100)
            .with_signer(BOB.clone())
            .build()
            .await;
        let tx101 = fixture
            .checked_tx_builder()
            .with_nonce(101)
            .with_signer(BOB.clone())
            .build()
            .await;
        mempool.insert(tx100.clone()).await.unwrap();
        mempool.insert(tx101.clone()).await.unwrap();

        assert_eq!(mempool.len().await, 4);

        // Check the pending nonces
        assert_eq!(
            mempool.pending_nonce(&ALICE_ADDRESS_BYTES).await.unwrap(),
            2
        );
        assert_eq!(
            mempool.pending_nonce(&BOB_ADDRESS_BYTES).await.unwrap(),
            102
        );

        // Check the pending nonce for an address with no txs is `None`.
        assert!(mempool.pending_nonce(&CAROL_ADDRESS_BYTES).await.is_none());
    }

    #[tokio::test]
    async fn tx_removal_cache() {
        let mut tx_cache = RemovalCache::new(NonZeroUsize::try_from(2).unwrap());

        let tx_0 = TransactionId::new([0u8; 32]);
        let tx_1 = TransactionId::new([1u8; 32]);
        let tx_2 = TransactionId::new([2u8; 32]);

        assert!(
            tx_cache.remove(&tx_0).is_none(),
            "no transaction should be cached at first"
        );

        tx_cache.add(tx_0, RemovalReason::Expired);
        assert!(
            tx_cache.remove(&tx_0).is_some(),
            "transaction was added, should be cached"
        );

        assert!(
            tx_cache.remove(&tx_0).is_none(),
            "transaction is cleared after reading"
        );

        tx_cache.add(tx_0, RemovalReason::Expired);
        tx_cache.add(tx_1, RemovalReason::Expired);
        tx_cache.add(tx_2, RemovalReason::Expired);
        assert!(
            tx_cache.remove(&tx_1).is_some(),
            "second transaction was added, should be cached"
        );
        assert!(
            tx_cache.remove(&tx_2).is_some(),
            "third transaction was added, should be cached"
        );
        assert!(
            tx_cache.remove(&tx_0).is_none(),
            "first transaction should not be cached"
        );
    }

    #[tokio::test]
    async fn tx_removal_cache_preserves_first_reason() {
        let mut tx_cache = RemovalCache::new(NonZeroUsize::try_from(2).unwrap());

        let tx_0 = TransactionId::new([0u8; 32]);

        tx_cache.add(tx_0, RemovalReason::Expired);
        tx_cache.add(tx_0, RemovalReason::LowerNonceInvalidated);

        assert!(
            matches!(tx_cache.remove(&tx_0), Some(RemovalReason::Expired)),
            "first removal reason should be preserved"
        );
    }

    #[tokio::test]
    async fn tx_tracked_invalid_removal_removes_all() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        let tx0 = new_alice_tx(&fixture, 0).await;
        let tx1 = new_alice_tx(&fixture, 1).await;

        // check that the parked transaction is in the tracked set
        mempool.insert(tx1.clone()).await.unwrap();
        assert!(mempool.is_tracked(tx1.id()).await);

        // check that the pending transaction is in the tracked set
        mempool.insert(tx0.clone()).await.unwrap();
        assert!(mempool.is_tracked(tx0.id()).await);

        // remove the transactions from the mempool, should remove both
        mempool
            .remove_tx_invalid(tx0.clone(), RemovalReason::Expired)
            .await;

        // check that the transactions are not in the tracked set
        assert!(!mempool.is_tracked(tx0.id()).await);
        assert!(!mempool.is_tracked(tx1.id()).await);
    }

    #[tokio::test]
    async fn tx_tracked_maintenance_removes_all() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        let tx0 = new_alice_tx(&fixture, 0).await;
        let tx1 = new_alice_tx(&fixture, 1).await;

        mempool.insert(tx1.clone()).await.unwrap();
        mempool.insert(tx0.clone()).await.unwrap();

        // remove the transactions from the mempool via maintenance
        put_alice_nonce(&fixture, 2).await;
        mempool
            .run_maintenance(fixture.storage().latest_snapshot(), false, HashMap::new())
            .await;

        // check that the transactions are not in the tracked set
        assert!(!mempool.is_tracked(tx0.id()).await);
        assert!(!mempool.is_tracked(tx1.id()).await);
    }

    #[tokio::test]
    async fn tx_tracked_reinsertion_ok() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        let tx0 = new_alice_tx(&fixture, 0).await;
        let tx1 = new_alice_tx(&fixture, 1).await;
        mempool.insert(tx1.clone()).await.unwrap();
        mempool.insert(tx0.clone()).await.unwrap();

        // remove the transactions from the mempool, should remove both
        mempool
            .remove_tx_invalid(tx0.clone(), RemovalReason::Expired)
            .await;

        assert!(!mempool.is_tracked(tx0.id()).await);
        assert!(!mempool.is_tracked(tx1.id()).await);

        // re-insert the transactions into the mempool
        mempool.insert(tx0.clone()).await.unwrap();
        mempool.insert(tx1.clone()).await.unwrap();

        // check that the transactions are in the tracked set on re-insertion
        assert!(mempool.is_tracked(tx0.id()).await);
        assert!(mempool.is_tracked(tx1.id()).await);
    }

    #[tokio::test]
    async fn transaction_still_exists_in_recently_included_after_being_removed() {
        const INCLUDED_TX_BLOCK_NUMBER: u64 = 42;
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        // Create and insert transactions
        put_alice_nonce(&fixture, 1).await;
        let tx_1 = new_alice_tx(&fixture, 1).await;
        let tx_2 = new_alice_tx(&fixture, 2).await;
        mempool.insert(tx_1.clone()).await.unwrap();
        mempool.insert(tx_2.clone()).await.unwrap();

        // Check that transactions are in pending state
        assert_tx_in_pending(&mempool, tx_1.id()).await;
        assert_tx_in_pending(&mempool, tx_2.id()).await;

        // Setup state for maintenance
        put_alice_nonce(&fixture, 3).await;
        put_block_height(&fixture, INCLUDED_TX_BLOCK_NUMBER).await;

        // Create the transaction result to be used in the execution result
        let exec_result_1 = Arc::new(ExecTxResult {
            log: "tx_1 executed".to_string(),
            ..ExecTxResult::default()
        });
        let exec_result_2 = Arc::new(ExecTxResult {
            log: "tx_2 executed".to_string(),
            ..ExecTxResult::default()
        });

        // Remove transactions as included in a block
        let mut execution_results = HashMap::new();
        execution_results.insert(*tx_1.id(), exec_result_1.clone());
        execution_results.insert(*tx_2.id(), exec_result_2.clone());
        mempool
            .run_maintenance(
                fixture.storage().latest_snapshot(),
                false,
                execution_results,
            )
            .await;
        let removal_cache = mempool.removal_cache().await;
        assert_eq!(removal_cache.len(), 2, "removal cache should have 2 txs");

        let expected_removal_reason_1 = RemovalReason::IncludedInBlock {
            height: INCLUDED_TX_BLOCK_NUMBER,
            result: exec_result_1,
        };
        let expected_removal_reason_2 = RemovalReason::IncludedInBlock {
            height: INCLUDED_TX_BLOCK_NUMBER,
            result: exec_result_2,
        };
        assert_tx_removed(&mempool, tx_1.id(), &expected_removal_reason_1).await;
        assert_tx_removed(&mempool, tx_2.id(), &expected_removal_reason_2).await;

        // Remove actions from removal cache to simulate recheck
        mempool.remove_from_removal_cache(tx_1.id()).await;
        mempool.remove_from_removal_cache(tx_2.id()).await;
        let removal_cache = mempool.removal_cache().await;
        assert!(removal_cache.is_empty(), "removal cache should be empty");

        // Check that transaction status is still removed with "included" reason
        assert_tx_removed(&mempool, tx_1.id(), &expected_removal_reason_1).await;
        assert_tx_removed(&mempool, tx_2.id(), &expected_removal_reason_2).await;
    }

    #[tokio::test]
    async fn transaction_status_none_after_recently_included_expiration() {
        use tokio::time;

        const INCLUDED_TX_BLOCK_NUMBER: u64 = 42;
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        // Create and insert a transaction
        let tx = new_alice_tx(&fixture, 1).await;
        mempool.insert(tx.clone()).await.unwrap();

        // Setup state for maintenance
        put_alice_nonce(&fixture, 2).await;
        put_block_height(&fixture, INCLUDED_TX_BLOCK_NUMBER).await;

        // Mark transaction as included in a block
        let exec_result = Arc::new(ExecTxResult {
            log: "tx executed".to_string(),
            ..ExecTxResult::default()
        });
        let mut execution_results = HashMap::new();
        execution_results.insert(*tx.id(), exec_result.clone());
        mempool
            .run_maintenance(
                fixture.storage().latest_snapshot(),
                false,
                execution_results,
            )
            .await;

        let expected_removal_reason = RemovalReason::IncludedInBlock {
            height: INCLUDED_TX_BLOCK_NUMBER,
            result: exec_result,
        };
        assert_tx_removed(&mempool, tx.id(), &expected_removal_reason).await;

        // Advance time to expire the transaction in the `recently_included_transactions` cache
        time::pause();
        time::advance(Duration::from_secs(61)).await;

        // Remove from CometBFT removal cache
        mempool.remove_from_removal_cache(tx.id()).await;
        // Maintenance should remove from recently included transactions
        mempool
            .run_maintenance(fixture.storage().latest_snapshot(), false, HashMap::new())
            .await;

        assert!(
            mempool.transaction_status(tx.id()).await.is_none(),
            "Transaction status should be None after expiration from recently included cache"
        );
    }

    #[tokio::test]
    async fn parked_limit_enforced() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        for nonce in 1..=u32::try_from(MAX_PARKED_TXS_PER_ACCOUNT).unwrap() {
            let tx = new_alice_tx(&fixture, nonce).await;
            mempool.insert(tx).await.unwrap();
        }

        // size limit fails as expected
        let tx = new_alice_tx(
            &fixture,
            u32::try_from(MAX_PARKED_TXS_PER_ACCOUNT)
                .unwrap()
                .saturating_add(1),
        )
        .await;
        assert!(
            matches!(
                mempool.insert(tx).await.unwrap_err(),
                InsertionError::AccountSizeLimit
            ),
            "size limit should be enforced"
        );
    }

    #[tokio::test]
    async fn run_maintenance_included() {
        const INCLUDED_TX_BLOCK_NUMBER: u64 = 12;

        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        put_alice_nonce(&fixture, 1).await;
        let tx1 = new_alice_tx(&fixture, 1).await;
        let tx2 = new_alice_tx(&fixture, 2).await;
        let tx3 = new_alice_tx(&fixture, 3).await;
        let tx4 = new_alice_tx(&fixture, 4).await;
        mempool.insert(tx1.clone()).await.unwrap();
        mempool.insert(tx2.clone()).await.unwrap();
        mempool.insert(tx3.clone()).await.unwrap();
        mempool.insert(tx4.clone()).await.unwrap();

        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue,
            vec![tx1.clone(), tx2.clone(), tx3.clone(), tx4.clone()]
        );

        // setup state as if tx1 and tx2 have been executed
        put_alice_nonce(&fixture, 3).await;
        put_block_height(&fixture, INCLUDED_TX_BLOCK_NUMBER).await;
        let mut execution_results = HashMap::new();
        execution_results.insert(*tx1.id(), Arc::new(ExecTxResult::default()));
        execution_results.insert(*tx2.id(), Arc::new(ExecTxResult::default()));

        mempool
            .run_maintenance(
                fixture.storage().latest_snapshot(),
                false,
                execution_results,
            )
            .await;

        // builder queue should now contain only unexecuted txs
        let builder_queue = mempool.builder_queue().await;
        assert_eq!(builder_queue, vec![tx3.clone(), tx4.clone()]);

        let expected_reason = RemovalReason::IncludedInBlock {
            height: INCLUDED_TX_BLOCK_NUMBER,
            result: Arc::new(ExecTxResult::default()),
        };
        assert_tx_removed(&mempool, tx1.id(), &expected_reason).await;
        assert_tx_removed(&mempool, tx2.id(), &expected_reason).await;
    }

    #[tokio::test]
    async fn insert_promoted_tx_removed_if_its_insertion_fails() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        let pending_tx_1 =
            TimemarkedTransaction::new(new_alice_tx(&fixture, 1).await, HashMap::new());
        let pending_tx_2 = new_alice_tx(&fixture, 2).await;
        // different rollup data so that this transaction's hash is different than the failing tx
        let pending_tx_3 = TimemarkedTransaction::new(
            fixture
                .checked_tx_builder()
                .with_nonce(3)
                .with_rollup_data_submission(vec![2, 3, 4])
                .with_signer(ALICE.clone())
                .build()
                .await,
            HashMap::new(),
        );
        let failure_tx =
            TimemarkedTransaction::new(new_alice_tx(&fixture, 3).await, HashMap::new());

        let mut inner = mempool.inner.write().await;

        // Add tx nonce 1 into pending
        inner
            .pending
            .add(pending_tx_1.clone(), 1, &HashMap::new())
            .unwrap();

        // Force transactions with nonce 3 into both pending and parked, such that the parked one
        // will fail on promotion
        inner
            .parked
            .add(failure_tx.clone(), 1, &HashMap::new())
            .unwrap();
        inner
            .pending
            .add(pending_tx_3.clone(), 3, &HashMap::new())
            .unwrap();

        inner.contained_txs.insert(*pending_tx_1.id());
        inner.contained_txs.insert(*failure_tx.id());
        inner.contained_txs.insert(*pending_tx_3.id());

        assert_eq!(inner.comet_bft_removal_cache.cache.len(), 0);
        assert_eq!(inner.contained_txs.len(), 3);

        drop(inner);

        // Insert tx nonce 2 to mempool, prompting promotion of tx nonce 3 from parked to pending,
        // which should fail
        mempool.insert(pending_tx_2).await.unwrap();

        let inner = mempool.inner.read().await;

        assert_eq!(inner.comet_bft_removal_cache.cache.len(), 1);
        assert!(
            inner
                .comet_bft_removal_cache
                .cache
                .contains_key(failure_tx.id()),
            "CometBFT removal cache should contain the failed tx"
        );
        assert_eq!(inner.contained_txs.len(), 3);
        assert!(
            !inner.contained_txs.contains(failure_tx.id()),
            "contained txs should not contain the failed tx id"
        );
    }
}
