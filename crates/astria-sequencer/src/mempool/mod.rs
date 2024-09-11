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
    primitive::v1::asset::IbcPrefixed,
    protocol::transaction::v1alpha1::SignedTransaction,
};
pub(crate) use mempool_state::get_account_balances;
use tokio::{
    join,
    sync::{
        RwLock,
        RwLockWriteGuard,
    },
    time::Duration,
};
use tracing::{
    error,
    instrument,
};
pub(crate) use transactions_container::InsertionError;
use transactions_container::{
    ParkedTransactions,
    PendingTransactions,
    TimemarkedTransaction,
};

use crate::accounts;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum RemovalReason {
    Expired,
    NonceStale,
    LowerNonceInvalidated,
    FailedPrepareProposal(String),
}

/// How long transactions are considered valid in the mempool.
const TX_TTL: Duration = Duration::from_secs(240);
/// Max number of parked transactions allowed per account.
const MAX_PARKED_TXS_PER_ACCOUNT: usize = 15;
/// Max number of transactions to keep in the removal cache. Should be larger than the max number of
/// transactions allowed in the cometBFT mempool.
const REMOVAL_CACHE_SIZE: usize = 4096;

/// `RemovalCache` is used to signal to `CometBFT` that a
/// transaction can be removed from the `CometBFT` mempool.
///
/// This is useful for when a transaction fails execution or when
/// a transaction is invalidated due to mempool removal policies.
#[derive(Clone)]
pub(crate) struct RemovalCache {
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
            // make space for the new transaction by removing the oldest transaction
            let removed_tx = self
                .remove_queue
                .pop_front()
                .expect("cache should contain elements");
            // remove transaction from cache if it is present
            self.cache.remove(&removed_tx);
        }
        self.remove_queue.push_back(tx_hash);
        self.cache.insert(tx_hash, reason);
    }
}

/// [`Mempool`] is an account-based structure for maintaining transactions for execution.
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
    pending: Arc<RwLock<PendingTransactions>>,
    parked: Arc<RwLock<ParkedTransactions<MAX_PARKED_TXS_PER_ACCOUNT>>>,
    comet_bft_removal_cache: Arc<RwLock<RemovalCache>>,
}

impl Mempool {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(PendingTransactions::new(TX_TTL))),
            parked: Arc::new(RwLock::new(ParkedTransactions::new(TX_TTL))),
            comet_bft_removal_cache: Arc::new(RwLock::new(RemovalCache::new(
                NonZeroUsize::try_from(REMOVAL_CACHE_SIZE)
                    .expect("Removal cache cannot be zero sized"),
            ))),
        }
    }

    /// Returns the number of transactions in the mempool.
    #[must_use]
    #[instrument(skip_all)]
    pub(crate) async fn len(&self) -> usize {
        #[rustfmt::skip]
        let (pending_len, parked_len) = join!(
            async { self.pending.read().await.len() },
            async { self.parked.read().await.len() }
        );
        pending_len.saturating_add(parked_len)
    }

    /// Inserts a transaction into the mempool and does not allow for transaction replacement.
    /// Will return the reason for insertion failure if failure occurs.
    #[instrument(skip_all)]
    pub(crate) async fn insert(
        &self,
        tx: Arc<SignedTransaction>,
        current_account_nonce: u32,
        current_account_balances: HashMap<IbcPrefixed, u128>,
        transaction_cost: HashMap<IbcPrefixed, u128>,
    ) -> anyhow::Result<(), InsertionError> {
        let timemarked_tx = TimemarkedTransaction::new(tx, transaction_cost);

        let (mut pending, mut parked) = self.acquire_both_locks().await;

        // try insert into pending
        match pending.add(
            timemarked_tx.clone(),
            current_account_nonce,
            &current_account_balances,
        ) {
            Err(InsertionError::NonceGap | InsertionError::AccountBalanceTooLow) => {
                // Release the lock asap.
                drop(pending);
                // try to add to parked queue
                parked.add(
                    timemarked_tx,
                    current_account_nonce,
                    &current_account_balances,
                )
            }
            error @ Err(
                InsertionError::AlreadyPresent
                | InsertionError::NonceTooLow
                | InsertionError::NonceTaken
                | InsertionError::AccountSizeLimit,
            ) => error,
            Ok(()) => {
                // check parked for txs able to be promoted
                let to_promote = parked.find_promotables(
                    timemarked_tx.address(),
                    timemarked_tx
                        .nonce()
                        .checked_add(1)
                        .expect("failed to increment nonce in promotion"),
                    &pending.subtract_contained_costs(
                        *timemarked_tx.address(),
                        current_account_balances.clone(),
                    ),
                );
                // promote the transactions
                for ttx in to_promote {
                    if let Err(error) =
                        pending.add(ttx, current_account_nonce, &current_account_balances)
                    {
                        error!(
                            current_account_nonce,
                            "failed to promote transaction during insertion: {error:#}"
                        );
                    }
                }
                Ok(())
            }
        }
    }

    /// Returns a copy of all transactions and their hashes ready for execution, sorted first by the
    /// difference between a transaction and the account's current nonce and then by the time that
    /// the transaction was first seen by the appside mempool.
    pub(crate) async fn builder_queue<S: accounts::StateReadExt>(
        &self,
        state: &S,
    ) -> anyhow::Result<Vec<([u8; 32], Arc<SignedTransaction>)>> {
        self.pending.read().await.builder_queue(state).await
    }

    /// Removes the target transaction and all transactions for associated account with higher
    /// nonces.
    ///
    /// This function should only be used to remove invalid/failing transactions and not executed
    /// transactions. Executed transactions will be removed in the `run_maintenance()` function.
    pub(crate) async fn remove_tx_invalid(
        &self,
        signed_tx: Arc<SignedTransaction>,
        reason: RemovalReason,
    ) {
        let tx_hash = signed_tx.id().get();
        let address = signed_tx.verification_key().address_bytes();

        // Try to remove from pending.
        let removed_txs = match self.pending.write().await.remove(signed_tx) {
            Ok(mut removed_txs) => {
                // Remove all of parked.
                removed_txs.append(&mut self.parked.write().await.clear_account(&address));
                removed_txs
            }
            Err(signed_tx) => {
                // Not found in pending, try to remove from parked and if not found, just return.
                match self.parked.write().await.remove(signed_tx) {
                    Ok(removed_txs) => removed_txs,
                    Err(_) => return,
                }
            }
        };

        // Add all removed to removal cache for cometbft.
        let mut removal_cache = self.comet_bft_removal_cache.write().await;
        // Add the original tx first, since it will also be listed in `removed_txs`.  The second
        // attempt to add it inside the loop below will be a no-op.
        removal_cache.add(tx_hash, reason);
        for removed_tx in removed_txs {
            removal_cache.add(removed_tx, RemovalReason::LowerNonceInvalidated);
        }
    }

    /// Checks if a transaction was flagged to be removed from the `CometBFT` mempool. Will
    /// remove the transaction from the cache if it is present.
    #[instrument(skip_all)]
    pub(crate) async fn check_removed_comet_bft(&self, tx_hash: [u8; 32]) -> Option<RemovalReason> {
        self.comet_bft_removal_cache.write().await.remove(tx_hash)
    }

    /// Updates stored transactions to reflect current blockchain state. Will remove transactions
    /// that have stale nonces or are expired. Will also shift transation between pending and
    /// parked to relfect changes in account balances.
    ///
    /// All removed transactions are added to the CometBFT removal cache to aid with CometBFT
    /// mempool maintenance.
    #[instrument(skip_all)]
    pub(crate) async fn run_maintenance<S: accounts::StateReadExt>(&self, state: &S, recost: bool) {
        let (mut pending, mut parked) = self.acquire_both_locks().await;
        let mut removed_txs = Vec::<([u8; 32], RemovalReason)>::new();

        // To clean we need to:
        // 1.) remove stale and expired transactions
        // 2.) recost remaining transactions if needed
        // 3.) check if we have transactions in pending which need to be demoted due
        //     to balance decreases
        // 4.) if there were no demotions, check if parked has transactions we can
        //     promote

        let addresses: HashSet<[u8; 20]> = pending
            .addresses()
            .into_iter()
            .chain(parked.addresses())
            .collect();

        // TODO: Make this concurrent, all account state is separate with IO bound disk reads.
        for address in addresses {
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
                        address = %telemetry::display::base64(&address),
                        "failed to fetch account balances when cleaning accounts: {error:#}"
                    );
                    continue;
                }
            };

            // clean pending and parked of stale and expired
            removed_txs.extend(pending.clean_account_stale_expired(address, current_nonce));
            if recost {
                pending.recost_transactions(address, state).await;
            }

            removed_txs.extend(parked.clean_account_stale_expired(address, current_nonce));
            if recost {
                parked.recost_transactions(address, state).await;
            }

            // get transactions to demote from pending
            let demotion_txs = pending.find_demotables(address, &current_balances);

            if demotion_txs.is_empty() {
                // nothing to demote, check for transactions to promote
                let highest_pending_nonce = pending
                    .pending_nonce(address)
                    .map_or(current_nonce, |nonce| nonce.saturating_add(1));

                let remaining_balances =
                    pending.subtract_contained_costs(address, current_balances.clone());
                let promtion_txs =
                    parked.find_promotables(&address, highest_pending_nonce, &remaining_balances);

                for tx in promtion_txs {
                    if let Err(error) = pending.add(tx, current_nonce, &current_balances) {
                        error!(
                            current_nonce,
                            "failed to promote transaction during maintenance: {error:#}"
                        );
                    }
                }
            } else {
                // add demoted transactions to parked
                for tx in demotion_txs {
                    if let Err(err) = parked.add(tx, current_nonce, &current_balances) {
                        // this shouldn't happen
                        error!(
                               address = %telemetry::display::base64(&address),
                               "failed to demote transaction during maintenance: {err:#}"
                        );
                    }
                }
            }
        }
        // Release the locks asap.
        drop(parked);
        drop(pending);

        // add to removal cache for cometbft
        let mut removal_cache = self.comet_bft_removal_cache.write().await;
        for (tx_hash, reason) in removed_txs {
            removal_cache.add(tx_hash, reason);
        }
    }

    /// Returns the highest pending nonce for the given address if it exists in the mempool. Note:
    /// does not take into account gapped nonces in the parked queue. For example, if the
    /// pending queue for an account has nonces [0,1] and the parked queue has [3], [1] will be
    /// returned.
    #[instrument(skip_all)]
    pub(crate) async fn pending_nonce(&self, address: [u8; 20]) -> Option<u32> {
        self.pending.read().await.pending_nonce(address)
    }

    async fn acquire_both_locks(
        &self,
    ) -> (
        RwLockWriteGuard<PendingTransactions>,
        RwLockWriteGuard<ParkedTransactions<MAX_PARKED_TXS_PER_ACCOUNT>>,
    ) {
        let pending = self.pending.write().await;
        let parked = self.parked.write().await;
        (pending, parked)
    }
}

#[cfg(test)]
mod test {
    use astria_core::crypto::SigningKey;

    use super::*;
    use crate::app::test_utils::{
        mock_balances,
        mock_state_getter,
        mock_state_put_account_balances,
        mock_state_put_account_nonce,
        mock_tx,
        mock_tx_cost,
    };

    #[tokio::test]
    async fn insert() {
        let mempool = Mempool::new();
        let signing_key = SigningKey::from([1; 32]);
        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);

        // sign and insert nonce 1
        let tx1 = mock_tx(1, &signing_key, "test");
        assert!(
            mempool
                .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );

        // try to insert again
        assert_eq!(
            mempool
                .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .unwrap_err(),
            InsertionError::AlreadyPresent,
            "already present"
        );

        // try to replace nonce
        let tx1_replacement = mock_tx(1, &signing_key, "test_0");
        assert_eq!(
            mempool
                .insert(
                    tx1_replacement.clone(),
                    0,
                    account_balances.clone(),
                    tx_cost.clone()
                )
                .await
                .unwrap_err(),
            InsertionError::NonceTaken,
            "nonce replace not allowed"
        );

        // add too low nonce
        let tx0 = mock_tx(0, &signing_key, "test");
        assert_eq!(
            mempool
                .insert(tx0.clone(), 1, account_balances, tx_cost)
                .await
                .unwrap_err(),
            InsertionError::NonceTooLow,
            "nonce too low"
        );
    }

    #[tokio::test]
    async fn single_account_flow_extensive() {
        // This test tries to hit the more complex edges of the mempool with a single account.
        // The test adds the nonces [1,2,0,4], creates a builder queue with the account
        // nonce at 1, and then cleans the pool to nonce 4. This tests some of the
        // odder edge cases that can be hit if a node goes offline or fails to see
        // some transactions that other nodes include into their proposed blocks.

        let mempool = Mempool::new();
        let signing_key = SigningKey::from([1; 32]);
        let signing_address = signing_key.verification_key().address_bytes();
        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);

        // add nonces in odd order to trigger insertion promotion logic
        // sign and insert nonce 1
        let tx1 = mock_tx(1, &signing_key, "test");
        assert!(
            mempool
                .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );

        // sign and insert nonce 2
        let tx2 = mock_tx(2, &signing_key, "test");
        assert!(
            mempool
                .insert(tx2.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 2 transaction into mempool"
        );

        // sign and insert nonce 0
        let tx0 = mock_tx(0, &signing_key, "test");
        assert!(
            mempool
                .insert(tx0.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );

        // sign and insert nonce 4
        let tx4 = mock_tx(4, &signing_key, "test");
        assert!(
            mempool
                .insert(tx4.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 4 transaction into mempool"
        );

        // assert size
        assert_eq!(mempool.len().await, 4);

        // mock state with nonce at 1
        let mut mock_state = mock_state_getter().await;
        mock_state_put_account_nonce(&mut mock_state, signing_address, 1);

        // grab building queue, should return transactions [1,2] since [0] was below and [4] is
        // gapped
        let builder_queue = mempool
            .builder_queue(&mock_state)
            .await
            .expect("failed to get builder queue");

        // see contains first two transactions that should be pending
        assert_eq!(builder_queue[0].1.nonce(), 1, "nonce should be one");
        assert_eq!(builder_queue[1].1.nonce(), 2, "nonce should be two");

        // see mempool's transactions just cloned, not consumed
        assert_eq!(mempool.len().await, 4);

        // run maintenance with simulated nonce to remove the nonces 0,1,2 and promote 4 from parked
        // to pending

        // setup state
        mock_state_put_account_nonce(&mut mock_state, signing_address, 4);
        mock_state_put_account_balances(&mut mock_state, signing_address, mock_balances(100, 100));

        mempool.run_maintenance(&mock_state, false).await;

        // assert mempool at 1
        assert_eq!(mempool.len().await, 1);

        // see transaction [4] properly promoted
        let mut builder_queue = mempool
            .builder_queue(&mock_state)
            .await
            .expect("failed to get builder queue");
        let (_, returned_tx) = builder_queue.pop().expect("should return last transaction");
        assert_eq!(returned_tx.nonce(), 4, "nonce should be four");
    }

    #[tokio::test]
    async fn run_maintenance_promotion() {
        let mempool = Mempool::new();
        let signing_key = SigningKey::from([1; 32]);
        let signing_address = signing_key.verification_key().address_bytes();

        // create transaction setup to trigger promotions
        //
        // initially pending has single transaction
        let initial_balances = mock_balances(1, 0);
        let tx_cost = mock_tx_cost(1, 0, 0);
        let tx1 = mock_tx(1, &signing_key, "test");
        let tx2 = mock_tx(2, &signing_key, "test");
        let tx3 = mock_tx(3, &signing_key, "test");
        let tx4 = mock_tx(4, &signing_key, "test");

        mempool
            .insert(tx1.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx2.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx3.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx4.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // see pending only has one transaction
        let mut mock_state = mock_state_getter().await;
        mock_state_put_account_nonce(&mut mock_state, signing_address, 1);

        let builder_queue = mempool
            .builder_queue(&mock_state)
            .await
            .expect("failed to get builder queue");
        assert_eq!(
            builder_queue.len(),
            1,
            "builder queue should only contain single transaction"
        );

        // run maintenance with account containing balance for two more transactions

        // setup state
        mock_state_put_account_balances(&mut mock_state, signing_address, mock_balances(3, 0));

        mempool.run_maintenance(&mock_state, false).await;

        // see builder queue now contains them
        let builder_queue = mempool
            .builder_queue(&mock_state)
            .await
            .expect("failed to get builder queue");
        assert_eq!(
            builder_queue.len(),
            3,
            "builder queue should now have 3 transactions"
        );
    }

    #[allow(clippy::too_many_lines)]
    #[tokio::test]
    async fn run_maintenance_demotion() {
        let mempool = Mempool::new();
        let signing_key = SigningKey::from([1; 32]);
        let signing_address = signing_key.verification_key().address_bytes();

        // create transaction setup to trigger demotions
        //
        // initially pending has four transactions
        let initial_balances = mock_balances(4, 0);
        let tx_cost = mock_tx_cost(1, 0, 0);
        let tx1 = mock_tx(1, &signing_key, "test");
        let tx2 = mock_tx(2, &signing_key, "test");
        let tx3 = mock_tx(3, &signing_key, "test");
        let tx4 = mock_tx(4, &signing_key, "test");

        mempool
            .insert(tx1.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx2.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx3.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx4.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // see pending only has all transactions

        let mut mock_state = mock_state_getter().await;
        mock_state_put_account_nonce(&mut mock_state, signing_address, 1);

        let builder_queue = mempool
            .builder_queue(&mock_state)
            .await
            .expect("failed to get builder queue");
        assert_eq!(
            builder_queue.len(),
            4,
            "builder queue should only contain four transactions"
        );

        // setup state
        mock_state_put_account_balances(&mut mock_state, signing_address, mock_balances(1, 0));

        mempool.run_maintenance(&mock_state, false).await;

        // see builder queue now contains single transactions
        let builder_queue = mempool
            .builder_queue(&mock_state)
            .await
            .expect("failed to get builder queue");
        assert_eq!(
            builder_queue.len(),
            1,
            "builder queue should contain single transaction"
        );

        mock_state_put_account_nonce(&mut mock_state, signing_address, 1);
        mock_state_put_account_balances(&mut mock_state, signing_address, mock_balances(3, 0));

        mempool.run_maintenance(&mock_state, false).await;

        let builder_queue = mempool
            .builder_queue(&mock_state)
            .await
            .expect("failed to get builder queue");
        assert_eq!(
            builder_queue.len(),
            3,
            "builder queue should contain three transactions"
        );
    }

    #[tokio::test]
    async fn remove_invalid() {
        let mempool = Mempool::new();
        let signing_key = SigningKey::from([1; 32]);
        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 10);

        // sign and insert nonces 0,1 and 3,4,5
        let tx0 = mock_tx(0, &signing_key, "test");
        assert!(
            mempool
                .insert(tx0.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );
        let tx1 = mock_tx(1, &signing_key, "test");
        assert!(
            mempool
                .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );
        let tx3 = mock_tx(3, &signing_key, "test");
        assert!(
            mempool
                .insert(tx3.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 3 transaction into mempool"
        );
        let tx4 = mock_tx(4, &signing_key, "test");
        assert!(
            mempool
                .insert(tx4.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 4 transaction into mempool"
        );
        let tx5 = mock_tx(5, &signing_key, "test");
        assert!(
            mempool
                .insert(tx5.clone(), 0, account_balances.clone(), tx_cost.clone())
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
        assert!(matches!(
            mempool.check_removed_comet_bft(tx0.id().get()).await,
            Some(RemovalReason::FailedPrepareProposal(_))
        ));
        assert!(matches!(
            mempool.check_removed_comet_bft(tx1.id().get()).await,
            Some(RemovalReason::FailedPrepareProposal(_))
        ));
        assert!(matches!(
            mempool.check_removed_comet_bft(tx3.id().get()).await,
            Some(RemovalReason::LowerNonceInvalidated)
        ));
        assert!(matches!(
            mempool.check_removed_comet_bft(tx4.id().get()).await,
            Some(RemovalReason::FailedPrepareProposal(_))
        ));
        assert!(matches!(
            mempool.check_removed_comet_bft(tx5.id().get()).await,
            Some(RemovalReason::LowerNonceInvalidated)
        ));
    }

    #[tokio::test]
    async fn should_get_pending_nonce() {
        let mempool = Mempool::new();
        let signing_key_0 = SigningKey::from([1; 32]);
        let signing_key_1 = SigningKey::from([2; 32]);
        let signing_key_2 = SigningKey::from([3; 32]);
        let signing_address_0 = signing_key_0.verification_key().address_bytes();
        let signing_address_1 = signing_key_1.verification_key().address_bytes();
        let signing_address_2 = signing_key_2.verification_key().address_bytes();
        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);

        // sign and insert nonces 0,1
        let tx0 = mock_tx(0, &signing_key_0, "test");
        assert!(
            mempool
                .insert(tx0.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );
        let tx1 = mock_tx(1, &signing_key_0, "test");
        assert!(
            mempool
                .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );

        // sign and insert nonces 100, 101
        let tx100 = mock_tx(100, &signing_key_1, "test");
        assert!(
            mempool
                .insert(
                    tx100.clone(),
                    100,
                    account_balances.clone(),
                    tx_cost.clone()
                )
                .await
                .is_ok(),
            "should be able to insert nonce 100 transaction into mempool"
        );
        let tx101 = mock_tx(101, &signing_key_1, "test");
        assert!(
            mempool
                .insert(
                    tx101.clone(),
                    100,
                    account_balances.clone(),
                    tx_cost.clone()
                )
                .await
                .is_ok(),
            "should be able to insert nonce 101 transaction into mempool"
        );

        assert_eq!(mempool.len().await, 4);

        // Check the pending nonces
        assert_eq!(mempool.pending_nonce(signing_address_0).await.unwrap(), 1);
        assert_eq!(mempool.pending_nonce(signing_address_1).await.unwrap(), 101);

        // Check the pending nonce for an address with no txs is `None`.
        assert!(mempool.pending_nonce(signing_address_2).await.is_none());
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
}
