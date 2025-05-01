#[cfg(feature = "benchmark")]
mod benchmarks;
mod mempool_state;
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
    primitive::v1::{
        asset::IbcPrefixed,
        TRANSACTION_ID_LEN,
    },
    protocol::transaction::v1::Transaction,
};
use astria_eyre::eyre::Result;
pub(crate) use mempool_state::get_account_balances;
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
    Metrics,
};

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum RemovalReason {
    Expired,
    NonceStale,
    LowerNonceInvalidated,
    FailedPrepareProposal(String),
    IncludedInBlock(u64),
}

impl std::fmt::Display for RemovalReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemovalReason::Expired => write!(f, "expired"),
            RemovalReason::NonceStale => write!(f, "stale nonce"),
            RemovalReason::LowerNonceInvalidated => write!(f, "lower nonce invalidated"),
            RemovalReason::FailedPrepareProposal(reason) => {
                write!(f, "failed prepare proposal: {reason}")
            }
            RemovalReason::IncludedInBlock(block_height) => {
                write!(f, "included in sequencer block {block_height}")
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
#[derive(Clone)]
struct RemovalCache {
    cache: HashMap<[u8; 32], RemovalReason>,
    remove_queue: VecDeque<[u8; 32]>,
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
    fn remove(&mut self, tx_hash: [u8; 32]) -> Option<RemovalReason> {
        self.cache.remove(&tx_hash)
    }

    /// Adds the transaction to the cache, will preserve the original
    /// `RemovalReason` if already in the cache.
    fn add(&mut self, tx_hash: [u8; 32], reason: RemovalReason) {
        if self.cache.contains_key(&tx_hash) {
            return;
        };

        if self.remove_queue.len() == usize::from(self.max_size) {
            // This should not happen if `REMOVAL_CACHE_SIZE` is >= CometBFT's configured mempool
            // size.
            //
            // Make space for the new transaction by removing the oldest transaction.
            let removed_tx = self
                .remove_queue
                .pop_front()
                .expect("cache should contain elements");
            warn!(
                tx_hash = %telemetry::display::hex(&removed_tx),
                removal_cache_size = REMOVAL_CACHE_SIZE,
                "popped transaction from appside mempool removal cache, CometBFT will not remove \
                this transaction from its mempool - removal cache size possibly too low"
            );
            // Remove transaction from cache if it is present.
            self.cache.remove(&removed_tx);
        }
        self.remove_queue.push_back(tx_hash);
        self.cache.insert(tx_hash, reason);
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
    pub(crate) fn new(metrics: &'static Metrics, parked_max_tx_count: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(MempoolInner::new(metrics, parked_max_tx_count))),
        }
    }

    /// Returns the number of transactions in the mempool.
    #[must_use]
    pub(crate) async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// Inserts a transaction into the mempool and does not allow for transaction replacement.
    /// Will return the reason for insertion failure if failure occurs.
    #[instrument(skip_all, fields(tx_hash = %tx.id(), current_account_nonce), err(level = Level::DEBUG))]
    pub(crate) async fn insert(
        &self,
        tx: Arc<Transaction>,
        current_account_nonce: u32,
        current_account_balances: &HashMap<IbcPrefixed, u128>,
        transaction_cost: HashMap<IbcPrefixed, u128>,
    ) -> Result<InsertionStatus, InsertionError> {
        self.inner.write().await.insert(
            tx,
            current_account_nonce,
            current_account_balances,
            transaction_cost,
        )
    }

    /// Returns a copy of all transactions and their hashes ready for execution, sorted first by the
    /// difference between a transaction and the account's current nonce and then by the time that
    /// the transaction was first seen by the appside mempool.
    pub(crate) async fn builder_queue(&self) -> Vec<([u8; 32], Arc<Transaction>)> {
        self.inner.read().await.builder_queue()
    }

    /// Removes the target transaction and all transactions for associated account with higher
    /// nonces.
    ///
    /// This function should only be used to remove invalid/failing transactions and not executed
    /// transactions. Executed transactions will be removed in the `run_maintenance()` function.
    #[instrument(skip_all, fields(tx_hash = %signed_tx.id()))]
    pub(crate) async fn remove_tx_invalid(
        &self,
        signed_tx: Arc<Transaction>,
        reason: RemovalReason,
    ) {
        self.inner
            .write()
            .await
            .remove_tx_invalid(signed_tx, reason);
    }

    /// Removes the transaction from the CometBFT removal cache if it is present.
    #[instrument(skip_all, fields(tx_hash = %hex::encode(tx_hash)))]
    pub(crate) async fn remove_from_removal_cache(&self, tx_hash: [u8; 32]) {
        self.inner.write().await.remove_from_removal_cache(tx_hash);
    }

    #[cfg(test)]
    pub(crate) async fn is_tracked(&self, tx_hash: [u8; 32]) -> bool {
        self.inner.read().await.is_tracked(tx_hash)
    }

    /// Updates stored transactions to reflect current blockchain state. Will remove transactions
    /// that have stale nonces or are expired. Will also shift transation between pending and
    /// parked to relfect changes in account balances.
    ///
    /// All removed transactions are added to the CometBFT removal cache to aid with CometBFT
    /// mempool maintenance.
    #[instrument(skip_all)]
    pub(crate) async fn run_maintenance<S: accounts::StateReadExt>(
        &self,
        state: &S,
        recost: bool,
        txs_included_in_block: HashSet<[u8; TRANSACTION_ID_LEN]>,
        block_number: u64,
    ) {
        self.inner
            .write()
            .await
            .run_maintenance(state, recost, txs_included_in_block, block_number)
            .await;
    }

    /// Returns the highest pending nonce for the given address if it exists in the mempool. Note:
    /// does not take into account gapped nonces in the parked queue. For example, if the
    /// pending queue for an account has nonces [0,1] and the parked queue has [3], [1] will be
    /// returned.
    #[instrument(skip_all, fields(address = %telemetry::display::base64(address)))]
    pub(crate) async fn pending_nonce(&self, address: &[u8; 20]) -> Option<u32> {
        self.inner.read().await.pending_nonce(address)
    }

    pub(crate) async fn transaction_status(
        &self,
        tx_hash: &[u8; TRANSACTION_ID_LEN],
    ) -> Option<TransactionStatus> {
        self.inner.read().await.transaction_status(tx_hash)
    }

    #[cfg(test)]
    pub(crate) async fn removal_cache(&self) -> HashMap<[u8; 32], RemovalReason> {
        self.inner.read().await.removal_cache()
    }
}

#[derive(Debug)]
pub(crate) enum InsertionStatus {
    AddedToParked,
    AddedToPending,
}

struct MempoolInner {
    pending: PendingTransactions,
    parked: ParkedTransactions<MAX_PARKED_TXS_PER_ACCOUNT>,
    comet_bft_removal_cache: RemovalCache,
    contained_txs: HashSet<[u8; 32]>,
    metrics: &'static Metrics,
}

impl MempoolInner {
    #[must_use]
    fn new(metrics: &'static Metrics, parked_max_tx_count: usize) -> Self {
        Self {
            pending: PendingTransactions::new(TX_TTL),
            parked: ParkedTransactions::new(TX_TTL, parked_max_tx_count),
            comet_bft_removal_cache: RemovalCache::new(
                NonZeroUsize::try_from(REMOVAL_CACHE_SIZE)
                    .expect("Removal cache cannot be zero sized"),
            ),
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
        tx: Arc<Transaction>,
        current_account_nonce: u32,
        current_account_balances: &HashMap<IbcPrefixed, u128>,
        transaction_cost: HashMap<IbcPrefixed, u128>,
    ) -> Result<InsertionStatus, InsertionError> {
        let timemarked_tx = TimemarkedTransaction::new(tx, transaction_cost);
        let id = timemarked_tx.id();

        // try insert into pending
        match self.pending.add(
            timemarked_tx.clone(),
            current_account_nonce,
            current_account_balances,
        ) {
            Err(InsertionError::NonceGap | InsertionError::AccountBalanceTooLow) => {
                // try to add to parked queue
                match self.parked.add(
                    timemarked_tx,
                    current_account_nonce,
                    current_account_balances,
                ) {
                    Ok(()) => {
                        // log current size of parked
                        self.metrics
                            .set_transactions_in_mempool_parked(self.parked.len());

                        // track in contained txs
                        self.contained_txs.insert(id);
                        Ok(InsertionStatus::AddedToParked)
                    }
                    Err(err) => Err(err),
                }
            }
            Err(error) => Err(error),
            Ok(()) => {
                // check parked for txs able to be promoted
                let to_promote = self.parked.find_promotables(
                    timemarked_tx.address(),
                    timemarked_tx
                        .nonce()
                        .checked_add(1)
                        .expect("failed to increment nonce in promotion"),
                    &self.pending.subtract_contained_costs(
                        timemarked_tx.address(),
                        current_account_balances.clone(),
                    ),
                );
                // promote the transactions
                for ttx in to_promote {
                    let tx_id = ttx.id();
                    if let Err(error) =
                        self.pending
                            .add(ttx, current_account_nonce, current_account_balances)
                    {
                        self.contained_txs.remove(&timemarked_tx.id());
                        error!(
                            current_account_nonce,
                            tx_hash = %telemetry::display::hex(&tx_id),
                            %error,
                            "failed to promote transaction during insertion"
                        );
                    }
                }

                // track in contained txs
                self.contained_txs.insert(timemarked_tx.id());

                Ok(InsertionStatus::AddedToPending)
            }
        }
    }

    fn builder_queue(&self) -> Vec<([u8; 32], Arc<Transaction>)> {
        self.pending.builder_queue()
    }

    fn remove_tx_invalid(&mut self, signed_tx: Arc<Transaction>, reason: RemovalReason) {
        let tx_hash = signed_tx.id().get();
        let address = *signed_tx.verification_key().address_bytes();

        // Try to remove from pending.
        let removed_txs = match self.pending.remove(signed_tx) {
            Ok(mut removed_txs) => {
                // Remove all of parked.
                removed_txs.append(&mut self.parked.clear_account(&address));
                removed_txs
            }
            Err(signed_tx) => {
                // Not found in pending, try to remove from parked and if not found, just return.
                match self.parked.remove(signed_tx) {
                    Ok(removed_txs) => removed_txs,
                    Err(_) => return,
                }
            }
        };

        // Add the original tx first to preserve its reason for removal. The second
        // attempt to add it inside the loop below will be a no-op.
        self.comet_bft_removal_cache.add(tx_hash, reason);
        for removed_tx in removed_txs {
            self.contained_txs.remove(&removed_tx);
            self.comet_bft_removal_cache
                .add(removed_tx, RemovalReason::LowerNonceInvalidated);
        }
    }

    fn remove_from_removal_cache(&mut self, tx_hash: [u8; 32]) {
        self.comet_bft_removal_cache.remove(tx_hash);
    }

    #[cfg(test)]
    fn is_tracked(&self, tx_hash: [u8; 32]) -> bool {
        self.contained_txs.contains(&tx_hash)
    }

    async fn run_maintenance<S: accounts::StateReadExt>(
        &mut self,
        state: &S,
        recost: bool,
        txs_included_in_block: HashSet<[u8; TRANSACTION_ID_LEN]>,
        block_number: u64,
    ) {
        let mut removed_txs = Vec::<([u8; 32], RemovalReason)>::new();

        // To clean we need to:
        // 1.) remove stale and expired transactions
        // 2.) recost remaining transactions if needed
        // 3.) check if we have transactions in pending which need to be demoted due
        //     to balance decreases
        // 4.) if there were no demotions, check if parked has transactions we can
        //     promote

        let addresses: HashSet<[u8; 20]> = self
            .pending
            .addresses()
            .chain(self.parked.addresses())
            .copied()
            .collect();

        // TODO: Make this concurrent, all account state is separate with IO bound disk reads.
        for address in &addresses {
            // get current account state
            let current_nonce = match state.get_account_nonce(address).await {
                Ok(res) => res,
                Err(error) => {
                    error!(
                        address = %telemetry::display::base64(&address),
                        "failed to fetch account nonce when cleaning accounts: {error:#}"
                    );
                    continue;
                }
            };
            let current_balances = match get_account_balances(state, address).await {
                Ok(res) => res,
                Err(error) => {
                    error!(
                        address = %telemetry::display::base64(address),
                        "failed to fetch account balances when cleaning accounts: {error:#}"
                    );
                    continue;
                }
            };

            // clean pending and parked of stale and expired
            removed_txs.extend(self.pending.clean_account_stale_expired(
                address,
                current_nonce,
                &txs_included_in_block,
                block_number,
            ));
            if recost {
                self.pending.recost_transactions(address, state).await;
            }

            removed_txs.extend(self.parked.clean_account_stale_expired(
                address,
                current_nonce,
                &txs_included_in_block,
                block_number,
            ));
            if recost {
                self.parked.recost_transactions(address, state).await;
            }

            // get transactions to demote from pending
            let demotion_txs = self.pending.find_demotables(address, &current_balances);

            if demotion_txs.is_empty() {
                // nothing to demote, check for transactions to promote
                let pending_nonce = self
                    .pending
                    .pending_nonce(address)
                    .map_or(current_nonce, |nonce| nonce);

                let remaining_balances = self
                    .pending
                    .subtract_contained_costs(address, current_balances.clone());
                let promotion_txs =
                    self.parked
                        .find_promotables(address, pending_nonce, &remaining_balances);

                for tx in promotion_txs {
                    let tx_id = tx.id();
                    if let Err(error) = self.pending.add(tx, current_nonce, &current_balances) {
                        self.contained_txs.remove(&tx_id);
                        self.metrics.increment_internal_logic_error();
                        error!(
                            address = %telemetry::display::base64(&address),
                            current_nonce,
                            tx_hash = %telemetry::display::hex(&tx_id),
                            %error,
                            "failed to promote transaction during maintenance"
                        );
                    }
                }
            } else {
                // add demoted transactions to parked
                for tx in demotion_txs {
                    let tx_id = tx.id();
                    if let Err(error) = self.parked.add(tx, current_nonce, &current_balances) {
                        self.contained_txs.remove(&tx_id);
                        self.metrics.increment_internal_logic_error();
                        error!(
                            address = %telemetry::display::base64(&address),
                            current_nonce,
                            tx_hash = %telemetry::display::hex(&tx_id),
                            %error,
                            "failed to demote transaction during maintenance"
                        );
                    }
                }
            }
        }

        // add to removal cache for cometbft and remove from the tracked set
        for (tx_hash, reason) in removed_txs {
            self.comet_bft_removal_cache.add(tx_hash, reason);
            self.contained_txs.remove(&tx_hash);
        }
    }

    fn pending_nonce(&self, address: &[u8; 20]) -> Option<u32> {
        self.pending.pending_nonce(address)
    }

    fn transaction_status(&self, tx_hash: &[u8; TRANSACTION_ID_LEN]) -> Option<TransactionStatus> {
        if self.contained_txs.contains(tx_hash) {
            if self.pending.contains_tx(tx_hash) {
                Some(TransactionStatus::Pending)
            } else {
                Some(TransactionStatus::Parked)
            }
        } else if let Some(reason) = self.comet_bft_removal_cache.cache.get(tx_hash) {
            match reason {
                RemovalReason::IncludedInBlock(block_number) => {
                    Some(TransactionStatus::IncludedInBlock(*block_number))
                }
                _ => Some(TransactionStatus::Removed(reason.clone())),
            }
        } else {
            None
        }
    }

    #[cfg(test)]
    fn removal_cache(&self) -> HashMap<[u8; 32], RemovalReason> {
        self.comet_bft_removal_cache.cache.clone()
    }
}

pub(crate) enum TransactionStatus {
    Pending,
    Parked,
    Removed(RemovalReason),
    IncludedInBlock(u64),
}

#[cfg(test)]
mod tests {
    use telemetry::Metrics;

    use super::*;
    use crate::{
        app::{
            benchmark_and_test_utils::{
                mock_balances,
                mock_state_getter,
                mock_state_put_account_balances,
                mock_state_put_account_nonce,
                mock_tx_cost,
                ALICE_ADDRESS,
                BOB_ADDRESS,
                CAROL_ADDRESS,
            },
            test_utils::{
                get_bob_signing_key,
                MockTxBuilder,
            },
        },
        benchmark_and_test_utils::astria_address_from_hex_string,
    };

    #[tokio::test]
    async fn insert() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);

        // sign and insert nonce 1
        let tx1 = MockTxBuilder::new().nonce(1).build();
        assert!(
            mempool
                .insert(tx1.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );
        assert_eq!(mempool.len().await, 1);

        // try to insert again
        assert_eq!(
            mempool
                .insert(tx1.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .unwrap_err(),
            InsertionError::AlreadyPresent,
            "already present"
        );

        // try to replace nonce
        let tx1_replacement = MockTxBuilder::new()
            .nonce(1)
            .chain_id("test-chain-id")
            .build();
        assert_eq!(
            mempool
                .insert(
                    tx1_replacement.clone(),
                    0,
                    &account_balances.clone(),
                    tx_cost.clone(),
                )
                .await
                .unwrap_err(),
            InsertionError::NonceTaken,
            "nonce replace not allowed"
        );

        // add too low nonce
        let tx0 = MockTxBuilder::new().nonce(0).build();
        assert_eq!(
            mempool
                .insert(tx0.clone(), 1, &account_balances, tx_cost)
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
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);

        // add nonces in odd order to trigger insertion promotion logic
        // sign and insert nonce 1
        let tx1 = MockTxBuilder::new().nonce(1).build();
        assert!(
            mempool
                .insert(tx1.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );

        // sign and insert nonce 2
        let tx2 = MockTxBuilder::new().nonce(2).build();
        assert!(
            mempool
                .insert(tx2.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 2 transaction into mempool"
        );

        // sign and insert nonce 0
        let tx0 = MockTxBuilder::new().nonce(0).build();
        assert!(
            mempool
                .insert(tx0.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );

        // sign and insert nonce 4
        let tx4 = MockTxBuilder::new().nonce(4).build();
        assert!(
            mempool
                .insert(tx4.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 4 transaction into mempool"
        );

        // assert size
        assert_eq!(mempool.len().await, 4);

        // grab building queue, should return transactions [0,1,2] since [4] is gapped
        let builder_queue = mempool.builder_queue().await;

        // see contains first two transactions that should be pending
        assert_eq!(builder_queue[0].1.nonce(), 0, "nonce should be zero");
        assert_eq!(builder_queue[1].1.nonce(), 1, "nonce should be one");
        assert_eq!(builder_queue[2].1.nonce(), 2, "nonce should be two");

        // see mempool's transactions just cloned, not consumed
        assert_eq!(mempool.len().await, 4);

        // run maintenance with simulated nonce to remove the nonces 0,1,2 and promote 4 from parked
        // to pending

        // setup state
        let mut mock_state = mock_state_getter().await;
        mock_state_put_account_nonce(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            4,
        );
        mock_state_put_account_balances(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            mock_balances(100, 100),
        );

        mempool
            .run_maintenance(&mock_state, false, HashSet::new(), 0)
            .await;

        // assert mempool at 1
        assert_eq!(mempool.len().await, 1);

        // see transaction [4] properly promoted
        let mut builder_queue = mempool.builder_queue().await;
        let (_, returned_tx) = builder_queue.pop().expect("should return last transaction");
        assert_eq!(returned_tx.nonce(), 4, "nonce should be four");
    }

    #[tokio::test]
    async fn run_maintenance_promotion() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        // create transaction setup to trigger promotions
        //
        // initially pending has single transaction
        let initial_balances = mock_balances(1, 0);
        let tx_cost = mock_tx_cost(1, 0, 0);
        let tx1 = MockTxBuilder::new().nonce(1).build();
        let tx2 = MockTxBuilder::new().nonce(2).build();
        let tx3 = MockTxBuilder::new().nonce(3).build();
        let tx4 = MockTxBuilder::new().nonce(4).build();

        mempool
            .insert(tx1.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx2.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx3.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx4.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // see pending only has one transaction
        let mut mock_state = mock_state_getter().await;
        mock_state_put_account_nonce(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            1,
        );

        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue.len(),
            1,
            "builder queue should only contain single transaction"
        );

        // run maintenance with account containing balance for two more transactions

        // setup state
        mock_state_put_account_balances(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            mock_balances(3, 0),
        );

        mempool
            .run_maintenance(&mock_state, false, HashSet::new(), 0)
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
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        // create transaction setup to trigger demotions
        //
        // initially pending has four transactions
        let initial_balances = mock_balances(4, 0);
        let tx_cost = mock_tx_cost(1, 0, 0);
        let tx1 = MockTxBuilder::new().nonce(1).build();
        let tx2 = MockTxBuilder::new().nonce(2).build();
        let tx3 = MockTxBuilder::new().nonce(3).build();
        let tx4 = MockTxBuilder::new().nonce(4).build();

        mempool
            .insert(tx1.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx2.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx3.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx4.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // see pending only has all transactions

        let mut mock_state = mock_state_getter().await;
        mock_state_put_account_nonce(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            1,
        );

        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue.len(),
            4,
            "builder queue should only contain four transactions"
        );

        // setup state
        mock_state_put_account_balances(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            mock_balances(1, 0),
        );

        mempool
            .run_maintenance(&mock_state, false, HashSet::new(), 0)
            .await;

        // see builder queue now contains single transactions
        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue.len(),
            1,
            "builder queue should contain single transaction"
        );

        mock_state_put_account_nonce(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            1,
        );
        mock_state_put_account_balances(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            mock_balances(3, 0),
        );

        mempool
            .run_maintenance(&mock_state, false, HashSet::new(), 0)
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
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 10);

        // sign and insert nonces 0,1 and 3,4,5
        let tx0 = MockTxBuilder::new().nonce(0).build();
        assert!(
            mempool
                .insert(tx0.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );
        let tx1 = MockTxBuilder::new().nonce(1).build();
        assert!(
            mempool
                .insert(tx1.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );
        let tx3 = MockTxBuilder::new().nonce(3).build();
        assert!(
            mempool
                .insert(tx3.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 3 transaction into mempool"
        );
        let tx4 = MockTxBuilder::new().nonce(4).build();
        assert!(
            mempool
                .insert(tx4.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 4 transaction into mempool"
        );
        let tx5 = MockTxBuilder::new().nonce(5).build();
        assert!(
            mempool
                .insert(tx5.clone(), 0, &account_balances.clone(), tx_cost.clone())
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
    }

    #[tokio::test]
    async fn should_get_pending_nonce() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);

        // sign and insert nonces 0,1
        let tx0 = MockTxBuilder::new().nonce(0).build();
        assert!(
            mempool
                .insert(tx0.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );
        let tx1 = MockTxBuilder::new().nonce(1).build();
        assert!(
            mempool
                .insert(tx1.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );

        // sign and insert nonces 100, 101
        let tx100 = MockTxBuilder::new()
            .nonce(100)
            .signer(get_bob_signing_key())
            .build();
        assert!(
            mempool
                .insert(
                    tx100.clone(),
                    100,
                    &account_balances.clone(),
                    tx_cost.clone(),
                )
                .await
                .is_ok(),
            "should be able to insert nonce 100 transaction into mempool"
        );
        let tx101 = MockTxBuilder::new()
            .nonce(101)
            .signer(get_bob_signing_key())
            .build();
        assert!(
            mempool
                .insert(
                    tx101.clone(),
                    100,
                    &account_balances.clone(),
                    tx_cost.clone(),
                )
                .await
                .is_ok(),
            "should be able to insert nonce 101 transaction into mempool"
        );

        assert_eq!(mempool.len().await, 4);

        // Check the pending nonces
        assert_eq!(
            mempool
                .pending_nonce(astria_address_from_hex_string(ALICE_ADDRESS).as_bytes())
                .await
                .unwrap(),
            2
        );
        assert_eq!(
            mempool
                .pending_nonce(astria_address_from_hex_string(BOB_ADDRESS).as_bytes())
                .await
                .unwrap(),
            102
        );

        // Check the pending nonce for an address with no txs is `None`.
        assert!(mempool
            .pending_nonce(astria_address_from_hex_string(CAROL_ADDRESS).as_bytes())
            .await
            .is_none());
    }

    #[tokio::test]
    async fn tx_removal_cache() {
        let mut tx_cache = RemovalCache::new(NonZeroUsize::try_from(2).unwrap());

        let tx_0 = [0u8; 32];
        let tx_1 = [1u8; 32];
        let tx_2 = [2u8; 32];

        assert!(
            tx_cache.remove(tx_0).is_none(),
            "no transaction should be cached at first"
        );

        tx_cache.add(tx_0, RemovalReason::Expired);
        assert!(
            tx_cache.remove(tx_0).is_some(),
            "transaction was added, should be cached"
        );

        assert!(
            tx_cache.remove(tx_0).is_none(),
            "transaction is cleared after reading"
        );

        tx_cache.add(tx_0, RemovalReason::Expired);
        tx_cache.add(tx_1, RemovalReason::Expired);
        tx_cache.add(tx_2, RemovalReason::Expired);
        assert!(
            tx_cache.remove(tx_1).is_some(),
            "second transaction was added, should be cached"
        );
        assert!(
            tx_cache.remove(tx_2).is_some(),
            "third transaction was added, should be cached"
        );
        assert!(
            tx_cache.remove(tx_0).is_none(),
            "first transaction should not be cached"
        );
    }

    #[tokio::test]
    async fn tx_removal_cache_preserves_first_reason() {
        let mut tx_cache = RemovalCache::new(NonZeroUsize::try_from(2).unwrap());

        let tx_0 = [0u8; 32];

        tx_cache.add(tx_0, RemovalReason::Expired);
        tx_cache.add(tx_0, RemovalReason::LowerNonceInvalidated);

        assert!(
            matches!(tx_cache.remove(tx_0), Some(RemovalReason::Expired)),
            "first removal reason should be presenved"
        );
    }

    #[tokio::test]
    async fn tx_tracked_invalid_removal_removes_all() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);

        let tx0 = MockTxBuilder::new().nonce(0).build();
        let tx1 = MockTxBuilder::new().nonce(1).build();

        // check that the parked transaction is in the tracked set
        mempool
            .insert(tx1.clone(), 0, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        assert!(mempool.is_tracked(tx1.id().get()).await);

        // check that the pending transaction is in the tracked set
        mempool
            .insert(tx0.clone(), 0, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        assert!(mempool.is_tracked(tx0.id().get()).await);

        // remove the transactions from the mempool, should remove both
        mempool
            .remove_tx_invalid(tx0.clone(), RemovalReason::Expired)
            .await;

        // check that the transactions are not in the tracked set
        assert!(!mempool.is_tracked(tx0.id().get()).await);
        assert!(!mempool.is_tracked(tx1.id().get()).await);
    }

    #[tokio::test]
    async fn tx_tracked_maintenance_removes_all() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);

        let tx0 = MockTxBuilder::new().nonce(0).build();
        let tx1 = MockTxBuilder::new().nonce(1).build();

        mempool
            .insert(tx1.clone(), 0, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx0.clone(), 0, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // remove the transacitons from the mempool via maintenance
        let mut mock_state = mock_state_getter().await;
        mock_state_put_account_nonce(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            2,
        );
        mempool
            .run_maintenance(&mock_state, false, HashSet::new(), 0)
            .await;

        // check that the transactions are not in the tracked set
        assert!(!mempool.is_tracked(tx0.id().get()).await);
        assert!(!mempool.is_tracked(tx1.id().get()).await);
    }

    #[tokio::test]
    async fn tx_tracked_reinsertion_ok() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);

        let tx0 = MockTxBuilder::new().nonce(0).build();
        let tx1 = MockTxBuilder::new().nonce(1).build();

        mempool
            .insert(tx1.clone(), 0, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        mempool
            .insert(tx0.clone(), 0, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // remove the transactions from the mempool, should remove both
        mempool
            .remove_tx_invalid(tx0.clone(), RemovalReason::Expired)
            .await;

        assert!(!mempool.is_tracked(tx0.id().get()).await);
        assert!(!mempool.is_tracked(tx1.id().get()).await);

        // re-insert the transactions into the mempool
        mempool
            .insert(tx0.clone(), 0, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx1.clone(), 0, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // check that the transactions are in the tracked set on re-insertion
        assert!(mempool.is_tracked(tx0.id().get()).await);
        assert!(mempool.is_tracked(tx1.id().get()).await);
    }

    #[tokio::test]
    async fn parked_limit_enforced() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 1);
        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);

        let tx0 = MockTxBuilder::new().nonce(1).build();
        let tx1 = MockTxBuilder::new().nonce(2).build();

        mempool
            .insert(tx1.clone(), 0, &account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // size limit fails as expected
        assert_eq!(
            mempool
                .insert(tx0.clone(), 0, &account_balances.clone(), tx_cost.clone())
                .await
                .unwrap_err(),
            InsertionError::ParkedSizeLimit,
            "size limit should be enforced"
        );
    }

    #[tokio::test]
    async fn run_maintenance_included() {
        const INCLUDED_TX_BLOCK_NUMBER: u64 = 12;

        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        let initial_balances = mock_balances(4, 0);
        let tx_cost = mock_tx_cost(0, 0, 0);
        let tx1 = MockTxBuilder::new().nonce(1).build();
        let tx2 = MockTxBuilder::new().nonce(2).build();
        let tx3 = MockTxBuilder::new().nonce(3).build();
        let tx4 = MockTxBuilder::new().nonce(4).build();

        mempool
            .insert(tx1.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx2.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx3.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx4.clone(), 1, &initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        let mut mock_state = mock_state_getter().await;
        mock_state_put_account_nonce(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            3,
        );

        let builder_queue = mempool.builder_queue().await;
        assert_eq!(
            builder_queue.len(),
            4,
            "builder queue should only contain four transactions"
        );

        mock_state_put_account_balances(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            mock_balances(0, 0),
        );

        let mut included_txs = HashSet::new();
        included_txs.insert(tx1.id().get());
        included_txs.insert(tx2.id().get());

        mempool
            .run_maintenance(&mock_state, false, included_txs, INCLUDED_TX_BLOCK_NUMBER)
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
            *removal_cache.get(&tx1.id().get()).unwrap(),
            RemovalReason::IncludedInBlock(INCLUDED_TX_BLOCK_NUMBER),
            "removal reason should be included"
        );
        assert_eq!(
            *removal_cache.get(&tx2.id().get()).unwrap(),
            RemovalReason::IncludedInBlock(INCLUDED_TX_BLOCK_NUMBER),
            "removal reason should be included"
        );
    }
}
