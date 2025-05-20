#[cfg(feature = "benchmark")]
mod benchmarks;
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
pub(crate) use mempool_state::get_account_balances;
use recent_execution_results::RecentExecutionResults;
use tendermint::abci::types::ExecTxResult;
use tokio::{
    sync::RwLock,
    time::Duration,
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
    accounts,
    accounts::AddressBytes as _,
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
        metrics: &'static Metrics,
        parked_max_tx_count: usize,
        execution_results_cache_size: usize,
    ) -> Self {
        Self {
            inner: Arc::new(RwLock::new(MempoolInner::new(
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
    #[instrument(
        skip_all,
        fields(tx_id = %checked_tx.id(), current_account_nonce),
        err(level = Level::DEBUG)
    )]
    pub(crate) async fn insert(
        &self,
        checked_tx: Arc<CheckedTransaction>,
        current_account_nonce: u32,
        current_account_balances: &HashMap<IbcPrefixed, u128>,
        transaction_cost: HashMap<IbcPrefixed, u128>,
    ) -> Result<InsertionStatus, InsertionError> {
        self.inner.write().await.insert(
            checked_tx,
            current_account_nonce,
            current_account_balances,
            transaction_cost,
        )
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
    pub(crate) async fn run_maintenance<S: accounts::StateReadExt>(
        &self,
        state: &S,
        recost: bool,
        block_execution_results: HashMap<TransactionId, Arc<ExecTxResult>>,
        block_height: u64,
    ) {
        self.inner
            .write()
            .await
            .run_maintenance(state, recost, block_execution_results, block_height)
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
    metrics: &'static Metrics,
}

impl MempoolInner {
    #[must_use]
    fn new(
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
            metrics,
        }
    }

    #[must_use]
    fn len(&self) -> usize {
        self.contained_txs.len()
    }

    fn insert(
        &mut self,
        checked_tx: Arc<CheckedTransaction>,
        current_account_nonce: u32,
        current_account_balances: &HashMap<IbcPrefixed, u128>,
        transaction_costs: HashMap<IbcPrefixed, u128>,
    ) -> Result<InsertionStatus, InsertionError> {
        let ttx_to_insert = TimemarkedTransaction::new(checked_tx, transaction_costs);
        let tx_id_to_insert = *ttx_to_insert.id();

        // try insert into pending
        match self.pending.add(
            ttx_to_insert.clone(),
            current_account_nonce,
            current_account_balances,
        ) {
            Err(InsertionError::NonceGap | InsertionError::AccountBalanceTooLow) => {
                // try to add to parked queue
                match self.parked.add(
                    ttx_to_insert,
                    current_account_nonce,
                    current_account_balances,
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
                        current_account_balances,
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
    async fn run_maintenance<S: accounts::StateReadExt>(
        &mut self,
        state: &S,
        recost: bool,
        block_execution_results: HashMap<TransactionId, Arc<ExecTxResult>>,
        block_height: u64,
    ) {
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
            let current_nonce = match state.get_account_nonce(address_bytes).await {
                Ok(res) => res,
                Err(error) => {
                    error!(
                        address = %telemetry::display::base64(&address_bytes),
                        "failed to fetch account nonce when cleaning accounts: {error:#}"
                    );
                    continue;
                }
            };
            let current_balances = match get_account_balances(state, address_bytes).await {
                Ok(res) => res,
                Err(error) => {
                    error!(
                        address = %telemetry::display::base64(address_bytes),
                        "failed to fetch account balances when cleaning accounts: {error:#}"
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
                self.pending.recost_transactions(address_bytes, state).await;
            }

            removed_txs.extend(self.parked.clean_account_stale_expired(
                address_bytes,
                current_nonce,
                &block_execution_results,
                block_height,
            ));
            if recost {
                self.parked.recost_transactions(address_bytes, state).await;
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

pub(crate) enum TransactionStatus {
    Pending,
    Parked,
    Removed(RemovalReason),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::StateWriteExt as _,
        assets::StateWriteExt as _,
        test_utils::{
            denom_0,
            denom_1,
            denom_2,
            denom_3,
            denom_4,
            denom_5,
            denom_6,
            dummy_balances,
            dummy_tx_costs,
            Fixture,
            ALICE,
            ALICE_ADDRESS_BYTES,
            BOB,
            BOB_ADDRESS_BYTES,
            CAROL_ADDRESS_BYTES,
        },
    };

    fn put_ibc_assets(fixture: &mut Fixture) {
        fixture
            .state_mut()
            .put_ibc_asset(denom_0().unwrap_trace_prefixed())
            .unwrap();
        fixture
            .state_mut()
            .put_ibc_asset(denom_1().unwrap_trace_prefixed())
            .unwrap();
        fixture
            .state_mut()
            .put_ibc_asset(denom_2().unwrap_trace_prefixed())
            .unwrap();
        fixture
            .state_mut()
            .put_ibc_asset(denom_3().unwrap_trace_prefixed())
            .unwrap();
        fixture
            .state_mut()
            .put_ibc_asset(denom_4().unwrap_trace_prefixed())
            .unwrap();
        fixture
            .state_mut()
            .put_ibc_asset(denom_5().unwrap_trace_prefixed())
            .unwrap();
        fixture
            .state_mut()
            .put_ibc_asset(denom_6().unwrap_trace_prefixed())
            .unwrap();
    }

    fn put_alice_balances(fixture: &mut Fixture, account_balances: HashMap<IbcPrefixed, u128>) {
        for (denom, balance) in account_balances {
            fixture
                .state_mut()
                .put_account_balance(&*ALICE_ADDRESS_BYTES, &denom, balance)
                .unwrap();
        }
    }

    async fn new_alice_tx(fixture: &Fixture, nonce: u32) -> Arc<CheckedTransaction> {
        fixture
            .checked_tx_builder()
            .with_signer(ALICE.clone())
            .with_nonce(nonce)
            .build()
            .await
    }

    #[tokio::test]
    async fn insert() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let account_balances = dummy_balances(100, 100);
        let tx_costs = dummy_tx_costs(10, 10, 0);

        // sign and insert nonce 1
        let tx1 = new_alice_tx(&fixture, 1).await;
        assert!(
            mempool
                .insert(tx1.clone(), 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );
        assert_eq!(mempool.len().await, 1);

        // try to insert again
        assert_eq!(
            mempool
                .insert(tx1, 0, &account_balances, tx_costs.clone())
                .await
                .unwrap_err(),
            InsertionError::AlreadyPresent,
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
        assert_eq!(
            mempool
                .insert(tx1_replacement, 0, &account_balances, tx_costs.clone(),)
                .await
                .unwrap_err(),
            InsertionError::NonceTaken,
            "nonce replace not allowed"
        );

        // add too low nonce
        let tx0 = new_alice_tx(&fixture, 0).await;
        assert_eq!(
            mempool
                .insert(tx0, 1, &account_balances, tx_costs)
                .await
                .unwrap_err(),
            InsertionError::NonceTooLow,
            "nonce too low"
        );
    }

    #[tokio::test]
    async fn single_account_flow_extensive() {
        // This test tries to hit the more complex edges of the mempool with a single account.
        // The test adds the nonces [1,2,0,4], creates a builder queue, and then cleans the pool to
        // nonce 4. This tests some of the odder edge cases that can be hit if a node goes offline
        // or fails to see some transactions that other nodes include into their proposed blocks.
        let mut fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let account_balances = dummy_balances(100, 100);
        let tx_costs = dummy_tx_costs(10, 10, 0);

        // add nonces in odd order to trigger insertion promotion logic
        // sign and insert nonce 1
        let tx1 = new_alice_tx(&fixture, 1).await;
        assert!(
            mempool
                .insert(tx1, 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );

        // sign and insert nonce 2
        let tx2 = new_alice_tx(&fixture, 2).await;
        assert!(
            mempool
                .insert(tx2, 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 2 transaction into mempool"
        );

        // sign and insert nonce 0
        let tx0 = new_alice_tx(&fixture, 0).await;
        assert!(
            mempool
                .insert(tx0, 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );

        // sign and insert nonce 4
        let tx4 = new_alice_tx(&fixture, 4).await;
        assert!(
            mempool
                .insert(tx4, 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 4 transaction into mempool"
        );

        // assert size
        assert_eq!(mempool.len().await, 4);

        // grab building queue, should return transactions [0,1,2] since [4] is gapped
        let builder_queue = mempool.builder_queue().await;

        // see contains first two transactions that should be pending
        assert_eq!(builder_queue[0].nonce(), 0, "nonce should be zero");
        assert_eq!(builder_queue[1].nonce(), 1, "nonce should be one");
        assert_eq!(builder_queue[2].nonce(), 2, "nonce should be two");

        // see mempool's transactions just cloned, not consumed
        assert_eq!(mempool.len().await, 4);

        // run maintenance with simulated nonce to remove the nonces 0,1,2 and promote 4 from parked
        // to pending

        // setup state
        fixture
            .state_mut()
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, 4)
            .unwrap();
        put_ibc_assets(&mut fixture);
        put_alice_balances(&mut fixture, dummy_balances(100, 100));

        mempool
            .run_maintenance(fixture.state(), false, HashMap::new(), 0)
            .await;

        // assert mempool at 1
        assert_eq!(mempool.len().await, 1);

        // see transaction [4] properly promoted
        let mut builder_queue = mempool.builder_queue().await;
        let returned_tx = builder_queue.pop().expect("should return last transaction");
        assert_eq!(returned_tx.nonce(), 4, "nonce should be four");
    }

    #[tokio::test]
    async fn run_maintenance_promotion() {
        let mut fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        // create transaction setup to trigger promotions
        //
        // initially pending has single transaction
        let initial_balances = dummy_balances(1, 0);
        let tx_costs = dummy_tx_costs(1, 0, 0);
        let tx1 = new_alice_tx(&fixture, 1).await;
        let tx2 = new_alice_tx(&fixture, 2).await;
        let tx3 = new_alice_tx(&fixture, 3).await;
        let tx4 = new_alice_tx(&fixture, 4).await;

        mempool
            .insert(tx1, 1, &initial_balances, tx_costs.clone())
            .await
            .unwrap();
        mempool
            .insert(tx2, 1, &initial_balances, tx_costs.clone())
            .await
            .unwrap();
        mempool
            .insert(tx3, 1, &initial_balances, tx_costs.clone())
            .await
            .unwrap();
        mempool
            .insert(tx4, 1, &initial_balances, tx_costs.clone())
            .await
            .unwrap();

        // see pending only has one transaction
        fixture
            .state_mut()
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, 1)
            .unwrap();
        put_ibc_assets(&mut fixture);

        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue.len(),
            1,
            "builder queue should only contain single transaction"
        );

        // run maintenance with account containing balance for two more transactions

        // setup state
        put_alice_balances(&mut fixture, dummy_balances(3, 0));

        mempool
            .run_maintenance(fixture.state(), false, HashMap::new(), 0)
            .await;

        // see builder queue now contains them
        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue.len(),
            3,
            "builder queue should now have 3 transactions"
        );
    }

    #[tokio::test]
    async fn run_maintenance_demotion() {
        let mut fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        // create transaction setup to trigger demotions
        //
        // initially pending has four transactions
        let initial_balances = dummy_balances(4, 0);
        let tx_costs = dummy_tx_costs(1, 0, 0);
        let tx1 = new_alice_tx(&fixture, 1).await;
        let tx2 = new_alice_tx(&fixture, 2).await;
        let tx3 = new_alice_tx(&fixture, 3).await;
        let tx4 = new_alice_tx(&fixture, 4).await;

        mempool
            .insert(tx1, 1, &initial_balances, tx_costs.clone())
            .await
            .unwrap();
        mempool
            .insert(tx2, 1, &initial_balances, tx_costs.clone())
            .await
            .unwrap();
        mempool
            .insert(tx3, 1, &initial_balances, tx_costs.clone())
            .await
            .unwrap();
        mempool
            .insert(tx4, 1, &initial_balances, tx_costs.clone())
            .await
            .unwrap();

        // see pending only has all transactions

        fixture
            .state_mut()
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, 1)
            .unwrap();
        put_ibc_assets(&mut fixture);

        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue.len(),
            4,
            "builder queue should only contain four transactions"
        );

        // setup state
        put_alice_balances(&mut fixture, dummy_balances(1, 0));

        mempool
            .run_maintenance(fixture.state(), false, HashMap::new(), 0)
            .await;

        // see builder queue now contains single transactions
        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue.len(),
            1,
            "builder queue should contain single transaction"
        );

        put_alice_balances(&mut fixture, dummy_balances(3, 0));

        mempool
            .run_maintenance(fixture.state(), false, HashMap::new(), 0)
            .await;

        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue.len(),
            3,
            "builder queue should contain three transactions"
        );
    }

    #[tokio::test]
    async fn remove_invalid() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let account_balances = dummy_balances(100, 100);
        let tx_costs = dummy_tx_costs(10, 10, 10);

        // sign and insert nonces 0,1 and 3,4,5
        let tx0 = new_alice_tx(&fixture, 0).await;
        assert!(
            mempool
                .insert(tx0.clone(), 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );
        let tx1 = new_alice_tx(&fixture, 1).await;
        assert!(
            mempool
                .insert(tx1.clone(), 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );
        let tx3 = new_alice_tx(&fixture, 3).await;
        assert!(
            mempool
                .insert(tx3.clone(), 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 3 transaction into mempool"
        );
        let tx4 = new_alice_tx(&fixture, 4).await;
        assert!(
            mempool
                .insert(tx4.clone(), 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 4 transaction into mempool"
        );
        let tx5 = new_alice_tx(&fixture, 5).await;
        assert!(
            mempool
                .insert(tx5.clone(), 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 5 transaction into mempool"
        );
        assert_eq!(mempool.len().await, 5);

        let removal_reason = RemovalReason::FailedPrepareProposal("reason".to_string());

        // remove 4, should remove 4 and 5
        mempool
            .remove_tx_invalid(tx4.clone(), removal_reason.clone())
            .await;
        assert_eq!(mempool.len().await, 3);

        // remove 4 again is also ok
        mempool
            .remove_tx_invalid(
                tx4.clone(),
                RemovalReason::NonceStale, // shouldn't be inserted into removal cache
            )
            .await;
        assert_eq!(mempool.len().await, 3);

        // remove 1, should remove 1 and 3
        mempool
            .remove_tx_invalid(tx1.clone(), removal_reason.clone())
            .await;
        assert_eq!(mempool.len().await, 1);

        // remove 0
        mempool
            .remove_tx_invalid(tx0.clone(), removal_reason.clone())
            .await;
        assert_eq!(mempool.len().await, 0);

        // assert that all were added to the cometbft removal cache
        // and the expected reasons were tracked
        let mut removal_cache = mempool.removal_cache().await;
        assert!(matches!(
            removal_cache.remove(tx0.id()),
            Some(RemovalReason::FailedPrepareProposal(_))
        ));
        assert!(matches!(
            removal_cache.remove(tx1.id()),
            Some(RemovalReason::FailedPrepareProposal(_))
        ));
        assert!(matches!(
            removal_cache.remove(tx3.id()),
            Some(RemovalReason::LowerNonceInvalidated)
        ));
        assert!(matches!(
            removal_cache.remove(tx4.id()),
            Some(RemovalReason::FailedPrepareProposal(_))
        ));
        assert!(matches!(
            removal_cache.remove(tx5.id()),
            Some(RemovalReason::LowerNonceInvalidated)
        ));
    }

    #[tokio::test]
    async fn should_get_pending_nonce() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        let account_balances = dummy_balances(100, 100);
        let tx_costs = dummy_tx_costs(10, 10, 0);

        // sign and insert nonces 0,1
        let tx0 = new_alice_tx(&fixture, 0).await;
        assert!(
            mempool
                .insert(tx0, 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );
        let tx1 = new_alice_tx(&fixture, 1).await;
        assert!(
            mempool
                .insert(tx1, 0, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );

        // sign and insert nonces 100, 101
        let tx100 = fixture
            .checked_tx_builder()
            .with_nonce(100)
            .with_signer(BOB.clone())
            .build()
            .await;
        assert!(
            mempool
                .insert(tx100, 100, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 100 transaction into mempool"
        );
        let tx101 = fixture
            .checked_tx_builder()
            .with_nonce(101)
            .with_signer(BOB.clone())
            .build()
            .await;
        assert!(
            mempool
                .insert(tx101, 100, &account_balances, tx_costs.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 101 transaction into mempool"
        );

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
            "first removal reason should be presenved"
        );
    }

    #[tokio::test]
    async fn tx_tracked_invalid_removal_removes_all() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let account_balances = dummy_balances(100, 100);
        let tx_costs = dummy_tx_costs(10, 10, 0);

        let tx0 = new_alice_tx(&fixture, 0).await;
        let tx1 = new_alice_tx(&fixture, 1).await;

        // check that the parked transaction is in the tracked set
        mempool
            .insert(tx1.clone(), 0, &account_balances, tx_costs.clone())
            .await
            .unwrap();
        assert!(mempool.is_tracked(tx1.id()).await);

        // check that the pending transaction is in the tracked set
        mempool
            .insert(tx0.clone(), 0, &account_balances, tx_costs.clone())
            .await
            .unwrap();
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
        let mut fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let account_balances = dummy_balances(100, 100);
        let tx_costs = dummy_tx_costs(10, 10, 0);

        let tx0 = new_alice_tx(&fixture, 0).await;
        let tx1 = new_alice_tx(&fixture, 1).await;

        mempool
            .insert(tx1.clone(), 0, &account_balances, tx_costs.clone())
            .await
            .unwrap();
        mempool
            .insert(tx0.clone(), 0, &account_balances, tx_costs.clone())
            .await
            .unwrap();

        // remove the transactions from the mempool via maintenance
        fixture
            .state_mut()
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, 2)
            .unwrap();
        mempool
            .run_maintenance(fixture.state(), false, HashMap::new(), 0)
            .await;

        // check that the transactions are not in the tracked set
        assert!(!mempool.is_tracked(tx0.id()).await);
        assert!(!mempool.is_tracked(tx1.id()).await);
    }

    #[tokio::test]
    async fn tx_tracked_reinsertion_ok() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let account_balances = dummy_balances(100, 100);
        let tx_costs = dummy_tx_costs(10, 10, 0);

        let tx0 = new_alice_tx(&fixture, 0).await;
        let tx1 = new_alice_tx(&fixture, 1).await;

        mempool
            .insert(tx1.clone(), 0, &account_balances, tx_costs.clone())
            .await
            .unwrap();

        mempool
            .insert(tx0.clone(), 0, &account_balances, tx_costs.clone())
            .await
            .unwrap();

        // remove the transactions from the mempool, should remove both
        mempool
            .remove_tx_invalid(tx0.clone(), RemovalReason::Expired)
            .await;

        assert!(!mempool.is_tracked(tx0.id()).await);
        assert!(!mempool.is_tracked(tx1.id()).await);

        // re-insert the transactions into the mempool
        mempool
            .insert(tx0.clone(), 0, &account_balances, tx_costs.clone())
            .await
            .unwrap();
        mempool
            .insert(tx1.clone(), 0, &account_balances, tx_costs.clone())
            .await
            .unwrap();

        // check that the transactions are in the tracked set on re-insertion
        assert!(mempool.is_tracked(tx0.id()).await);
        assert!(mempool.is_tracked(tx1.id()).await);
    }

    #[tokio::test]
    async fn transaction_still_exists_in_recently_included_after_being_removed() {
        const INCLUDED_TX_BLOCK_NUMBER: u64 = 42;
        let mut fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let account_balances = dummy_balances(100, 100);
        let tx_cost = dummy_tx_costs(10, 10, 0);

        // Create and insert transactions
        let tx_1 = new_alice_tx(&fixture, 1).await;
        let tx_2 = new_alice_tx(&fixture, 2).await;

        mempool
            .insert(tx_1.clone(), 1, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx_2.clone(), 1, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // Check that transactions are in pending state
        assert!(matches!(
            mempool.transaction_status(tx_1.id()).await.unwrap(),
            TransactionStatus::Pending
        ));
        assert!(matches!(
            mempool.transaction_status(tx_2.id()).await.unwrap(),
            TransactionStatus::Pending
        ));

        // Setup state for maintenance
        fixture
            .state_mut()
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, 3)
            .unwrap();
        put_ibc_assets(&mut fixture);

        // Create the transaction result to be used in the execution result
        let exec_result_1 = ExecTxResult {
            log: "tx_1 executed".to_string(),
            ..ExecTxResult::default()
        };
        let exec_result_2 = ExecTxResult {
            log: "tx_2 executed".to_string(),
            ..ExecTxResult::default()
        };

        // Remove transactions as included in a block
        let mut execution_results = HashMap::new();
        execution_results.insert(*tx_1.id(), Arc::new(exec_result_1.clone()));
        execution_results.insert(*tx_2.id(), Arc::new(exec_result_2.clone()));
        mempool
            .run_maintenance(
                fixture.state(),
                false,
                execution_results,
                INCLUDED_TX_BLOCK_NUMBER,
            )
            .await;

        let TransactionStatus::Removed(RemovalReason::IncludedInBlock {
            height: tx1_height,
            result: tx1_result,
        }) = mempool.transaction_status(tx_1.id()).await.unwrap()
        else {
            panic!("tx_1 not marked as included in block");
        };
        assert_eq!(tx1_height, INCLUDED_TX_BLOCK_NUMBER);
        assert_eq!(*tx1_result, exec_result_1);

        let TransactionStatus::Removed(RemovalReason::IncludedInBlock {
            height: tx2_height,
            result: tx2_result,
        }) = mempool.transaction_status(tx_2.id()).await.unwrap()
        else {
            panic!("tx_2 not marked as included in block");
        };
        assert_eq!(tx2_height, INCLUDED_TX_BLOCK_NUMBER);
        assert_eq!(*tx2_result, exec_result_2);

        // Remove actions from removal cache to simulate recheck
        mempool.remove_from_removal_cache(tx_1.id()).await;
        mempool.remove_from_removal_cache(tx_2.id()).await;
        let removal_cache = mempool.removal_cache().await;
        assert!(removal_cache.is_empty(), "removal cache should be empty");

        // Check that transaction status is still removed with "included" reason
        let TransactionStatus::Removed(RemovalReason::IncludedInBlock {
            height,
            result,
        }) = mempool.transaction_status(tx_1.id()).await.unwrap()
        else {
            panic!("tx_1 not marked as included in block");
        };
        assert_eq!(height, INCLUDED_TX_BLOCK_NUMBER);
        assert_eq!(*result, exec_result_1);

        let TransactionStatus::Removed(RemovalReason::IncludedInBlock {
            height,
            result,
        }) = mempool.transaction_status(tx_2.id()).await.unwrap()
        else {
            panic!("tx_2 not marked as included in block");
        };
        assert_eq!(height, INCLUDED_TX_BLOCK_NUMBER);
        assert_eq!(*result, exec_result_2);
    }

    #[tokio::test]
    async fn transaction_status_none_after_recently_included_expiration() {
        use tokio::time;

        const INCLUDED_TX_BLOCK_NUMBER: u64 = 42;
        let mut fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();
        let account_balances = dummy_balances(100, 100);
        let tx_cost = dummy_tx_costs(10, 10, 0);

        // Create and insert a transaction
        let tx = new_alice_tx(&fixture, 1).await;
        mempool
            .insert(tx.clone(), 1, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // Setup state for maintenance
        fixture
            .state_mut()
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, 2)
            .unwrap();
        put_ibc_assets(&mut fixture);

        // Mark transaction as included in a block
        let exec_result = ExecTxResult {
            log: "tx executed".to_string(),
            ..ExecTxResult::default()
        };
        let mut execution_results = HashMap::new();
        execution_results.insert(*tx.id(), Arc::new(exec_result.clone()));
        mempool
            .run_maintenance(
                fixture.state(),
                false,
                execution_results,
                INCLUDED_TX_BLOCK_NUMBER,
            )
            .await;

        let TransactionStatus::Removed(RemovalReason::IncludedInBlock {
            height,
            result,
        }) = mempool.transaction_status(tx.id()).await.unwrap()
        else {
            panic!("transaction not marked as included in block");
        };
        assert_eq!(height, INCLUDED_TX_BLOCK_NUMBER);
        assert_eq!(*result, exec_result);

        // Advance time to expire the transaction in the `recently_included_transactions` cache
        time::pause();
        time::advance(time::Duration::from_secs(61)).await;

        // Remove from CometBFT removal cache
        mempool.remove_from_removal_cache(tx.id()).await;
        // Maintenance should remove from recently included transactions
        mempool
            .run_maintenance(
                fixture.state(),
                false,
                HashMap::new(),
                INCLUDED_TX_BLOCK_NUMBER + 1,
            )
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
        let account_balances = dummy_balances(100, 100);
        let tx_costs = dummy_tx_costs(10, 10, 0);

        for nonce in 1..=u32::try_from(MAX_PARKED_TXS_PER_ACCOUNT).unwrap() {
            let tx = new_alice_tx(&fixture, nonce).await;
            mempool
                .insert(tx, 0, &account_balances, tx_costs.clone())
                .await
                .unwrap();
        }

        // size limit fails as expected
        let tx = new_alice_tx(
            &fixture,
            u32::try_from(MAX_PARKED_TXS_PER_ACCOUNT)
                .unwrap()
                .saturating_add(1),
        )
        .await;
        assert_eq!(
            mempool
                .insert(tx, 0, &account_balances, tx_costs.clone())
                .await
                .unwrap_err(),
            InsertionError::AccountSizeLimit,
            "size limit should be enforced"
        );
    }

    #[tokio::test]
    async fn run_maintenance_included() {
        const INCLUDED_TX_BLOCK_NUMBER: u64 = 12;

        let mut fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        let initial_balances = dummy_balances(4, 0);
        let tx_costs = dummy_tx_costs(0, 0, 0);
        let tx1 = new_alice_tx(&fixture, 1).await;
        let tx2 = new_alice_tx(&fixture, 2).await;
        let tx3 = new_alice_tx(&fixture, 3).await;
        let tx4 = new_alice_tx(&fixture, 4).await;

        mempool
            .insert(tx1.clone(), 1, &initial_balances.clone(), tx_costs.clone())
            .await
            .unwrap();
        mempool
            .insert(tx2.clone(), 1, &initial_balances.clone(), tx_costs.clone())
            .await
            .unwrap();
        mempool
            .insert(tx3, 1, &initial_balances.clone(), tx_costs.clone())
            .await
            .unwrap();
        mempool
            .insert(tx4, 1, &initial_balances.clone(), tx_costs.clone())
            .await
            .unwrap();

        fixture
            .state_mut()
            .put_account_nonce(&*ALICE_ADDRESS_BYTES, 3)
            .unwrap();

        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue.len(),
            4,
            "builder queue should only contain four transactions"
        );

        put_alice_balances(&mut fixture, dummy_balances(0, 0));

        let mut execution_results = HashMap::new();
        execution_results.insert(*tx1.id(), Arc::new(ExecTxResult::default()));
        execution_results.insert(*tx2.id(), Arc::new(ExecTxResult::default()));

        mempool
            .run_maintenance(
                fixture.state(),
                false,
                execution_results,
                INCLUDED_TX_BLOCK_NUMBER,
            )
            .await;

        // see builder queue now contains single transactions
        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue.len(),
            2,
            "builder queue should contain two transactions"
        );

        let removal_cache = mempool.removal_cache().await;
        assert_eq!(
            removal_cache.len(),
            2,
            "removal cache should contain two transactions"
        );
        assert_eq!(
            *removal_cache.get(tx1.id()).unwrap(),
            RemovalReason::IncludedInBlock {
                height: INCLUDED_TX_BLOCK_NUMBER,
                result: Arc::new(ExecTxResult::default())
            },
            "removal reason should be included"
        );
        assert_eq!(
            *removal_cache.get(tx2.id()).unwrap(),
            RemovalReason::IncludedInBlock {
                height: INCLUDED_TX_BLOCK_NUMBER,
                result: Arc::new(ExecTxResult::default())
            },
            "removal reason should be included"
        );
    }

    #[tokio::test]
    async fn insert_promoted_tx_removed_if_its_insertion_fails() {
        let fixture = Fixture::default_initialized().await;
        let mempool = fixture.mempool();

        let account_balances = dummy_balances(100, 100);
        let tx_costs = dummy_tx_costs(10, 10, 0);

        let pending_tx_1 =
            TimemarkedTransaction::new(new_alice_tx(&fixture, 1).await, tx_costs.clone());
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
            tx_costs.clone(),
        );
        let failure_tx =
            TimemarkedTransaction::new(new_alice_tx(&fixture, 3).await, tx_costs.clone());

        let mut inner = mempool.inner.write().await;

        // Add tx nonce 1 into pending
        inner
            .pending
            .add(pending_tx_1.clone(), 1, &account_balances)
            .unwrap();

        // Force transactions with nonce 3 into both pending and parked, such that the parked one
        // will fail on promotion
        inner
            .parked
            .add(failure_tx.clone(), 1, &account_balances)
            .unwrap();
        inner
            .pending
            .add(pending_tx_3.clone(), 3, &account_balances)
            .unwrap();

        inner.contained_txs.insert(*pending_tx_1.id());
        inner.contained_txs.insert(*failure_tx.id());
        inner.contained_txs.insert(*pending_tx_3.id());

        assert_eq!(inner.comet_bft_removal_cache.cache.len(), 0);
        assert_eq!(inner.contained_txs.len(), 3);

        drop(inner);

        // Insert tx nonce 2 to mempool, prompting promotion of tx nonce 3 from parked to pending,
        // which should fail
        mempool
            .insert(pending_tx_2, 2, &account_balances, tx_costs)
            .await
            .unwrap();

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
