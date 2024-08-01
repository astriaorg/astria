mod benchmarks;
mod transactions_container;

use std::{
    collections::{
        HashMap,
        HashSet,
        VecDeque,
    },
    future::Future,
    num::NonZeroUsize,
    sync::Arc,
};

use astria_core::protocol::transaction::v1alpha1::SignedTransaction;
use tokio::{
    sync::RwLock,
    time::Duration,
};
use tracing::instrument;
use transactions_container::{
    BuilderQueue,
    InsertionError,
    TimemarkedTransaction,
    TransactionContainer,
};

#[derive(Debug, Clone)]
pub(crate) enum RemovalReason {
    Expired,
    NonceStale,
    LowerNonceInvalidated,
    FailedPrepareProposal(String),
    FailedCheckTx(String),
}

const TX_TTL: Duration = Duration::from_secs(240); // How long transactions are considered valid in the mempool.
const PARKED_SIZE_LIMIT: usize = 15; // Max number of parked transactions allowed per account.
const PENDING_SIZE_LIMIT: usize = 0; // Placeholder, is not enforced.
const REMOVAL_CACHE_SIZE: usize = 4096; // Max number of transactions to keep in the removal cache. Should be larger than the max number of transactions allowed in the cometBFT mempool.

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

/// [`Mempool`] is an account-based structure for maintaining transactions
/// for execution.
///
/// The transactions are split between pending and parked, where pending
/// transactions are ready for execution and parked transactions could be
/// executable in the future.
///
/// The mempool exposes the pending transactions through `builder_queue()`,
/// which returns all pending transactions sorted by the difference between the
/// transaction nonce and the current account nonce, and then by time first
/// seen. These transactions are returned as a copy.
///
/// The mempool implements the following policies:
/// 1. Nonce replacement is not allowed.
/// 2. Accounts cannot have more than `PARKED_SIZE_LIMIT` transactions in their parked queues.
/// 3. There is no account limit on pending transactions.
/// 4. Transactions will expire and can be removed after `TX_TTL` time.
/// 5. If an account has a transaction removed for being invalid or expired, all transactions for
///    that account with a higher nonce can be removed as well. This is due to the fact that we do
///    not execute failing transactions, so a transaction 'failing' will mean that further account
///    nonces will not be able to execute either.
///
/// Future extensions to this mempool can include:
/// - maximum mempool size
/// - account balance aware pending queue
///
/// Note: when grabbing locks to hold, grab them in order of: all, pending, parked. This
/// is just a convention to prevent deadlocks.
#[derive(Clone)]
pub(crate) struct Mempool {
    all: Arc<RwLock<HashSet<[u8; 32]>>>,
    pending: Arc<RwLock<TransactionContainer>>,
    parked: Arc<RwLock<TransactionContainer>>,
    comet_bft_removal_cache: Arc<RwLock<RemovalCache>>,
}

impl Mempool {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            all: Arc::new(RwLock::new(HashSet::<[u8; 32]>::new())),
            pending: Arc::new(RwLock::new(TransactionContainer::new(
                true,
                PENDING_SIZE_LIMIT,
                TX_TTL,
            ))),
            parked: Arc::new(RwLock::new(TransactionContainer::new(
                false,
                PARKED_SIZE_LIMIT,
                TX_TTL,
            ))),
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
        self.all.read().await.len()
    }

    /// Inserts a transaction into the mempool and does not allow for transaction replacement.
    /// Will return the reason for insertion failure if failure occurs.
    #[instrument(skip_all)]
    pub(crate) async fn insert(
        &self,
        tx: SignedTransaction,
        current_account_nonce: u32,
    ) -> anyhow::Result<(), InsertionError> {
        let timemarked_tx = Arc::new(TimemarkedTransaction::new(tx));

        // check if already tracked
        if self.all.read().await.contains(&timemarked_tx.tx_hash()) {
            return Err(InsertionError::AlreadyPresent);
        }

        // grab needed locks in normal order
        let mut all = self.all.write().await;
        let mut pending = self.pending.write().await;
        let mut parked = self.parked.write().await;

        // try insert into pending (will fail if nonce is gapped or already present)
        let mut success = pending.add(timemarked_tx.clone(), current_account_nonce);

        match success {
            Err(InsertionError::NonceGap) => {
                // try to add to parked queue
                success = parked.add(timemarked_tx.clone(), current_account_nonce);
            }
            Err(
                InsertionError::AlreadyPresent
                | InsertionError::NonceTooLow
                | InsertionError::NonceTaken
                | InsertionError::AccountSizeLimit,
            ) => {
                // noop
            }
            Ok(()) => {
                // check parked for txs able to be promoted
                let to_promote = parked.pop_front_account(
                    timemarked_tx.address(),
                    timemarked_tx
                        .signed_tx()
                        .nonce()
                        .checked_add(1)
                        .expect("failed to increment nonce in promotion"),
                );
                for tx in to_promote {
                    assert!(
                        pending.add(tx, current_account_nonce).is_ok(),
                        "promotion should work"
                    );
                }
            }
        }

        if success.is_ok() {
            // track in all list if successfully added
            all.insert(timemarked_tx.tx_hash());
        }

        success
    }

    /// Returns a copy of all transactions ready for execution, sorted
    /// first by the difference between a transaction and the account's
    /// nonce and then by the time that the transaction was first
    /// seen by the appside mempool.
    pub(crate) async fn builder_queue<F, O>(
        &self,
        current_account_nonce_getter: F,
    ) -> anyhow::Result<BuilderQueue>
    where
        F: Fn([u8; 20]) -> O,
        O: Future<Output = anyhow::Result<u32>>,
    {
        self.pending
            .read()
            .await
            .builder_queue(current_account_nonce_getter)
            .await
    }

    /// Removes the target transaction and all transactions for associated account
    /// with higher nonces.
    ///
    /// This function should only be used to remove invalid/failing transactions and
    /// not executed transactions. Executed transactions will be removed in the `run_maintenance()`
    /// function.
    pub(crate) async fn remove_tx_invalid(
        &self,
        signed_tx: &SignedTransaction,
        reason: RemovalReason,
    ) -> Vec<Arc<TimemarkedTransaction>> {
        let ttx: Arc<TimemarkedTransaction> =
            Arc::new(TimemarkedTransaction::new(signed_tx.clone()));
        let mut removed_txs = Vec::<Arc<TimemarkedTransaction>>::new();

        if !self.all.read().await.contains(&ttx.tx_hash()) {
            return removed_txs;
        }

        // grab needed main locks in normal order
        let mut all = self.all.write().await;
        let mut pending = self.pending.write().await;
        let mut parked = self.parked.write().await;
        let mut removal_cache = self.comet_bft_removal_cache.write().await;

        // mark as invalid in removal cache
        removal_cache.add(ttx.tx_hash(), reason);

        // try remove from pending
        removed_txs.append(&mut pending.remove(&ttx, true));
        if removed_txs.is_empty() {
            // try remove transaction from parked
            removed_txs.append(&mut parked.remove(&ttx, true));
        } else {
            // remove all of parked
            removed_txs.append(&mut parked.clear_account(ttx.address()));
        }
        assert!(!removed_txs.is_empty(), "error in remove_tx_invalid logic"); // TODO: is it ok to keep these in?

        // remove all from tracked and add to removal cache for cometbft
        for tx in &removed_txs {
            all.remove(&tx.tx_hash());
            removal_cache.add(tx.tx_hash(), RemovalReason::LowerNonceInvalidated);
        }

        removed_txs
    }

    /// Checks if a transaction was flagged to be removed from the `CometBFT` mempool. Will
    /// remove the transaction from the cache if it is present.
    #[instrument(skip_all)]
    pub(crate) async fn check_removed_comet_bft(&self, tx_hash: [u8; 32]) -> Option<RemovalReason> {
        self.comet_bft_removal_cache.write().await.remove(tx_hash)
    }

    /// Updates stored transactions to reflect current blockchain state. Will remove transactions
    /// that have stale nonces and will remove transaction that are expired.
    ///
    /// All removed transactions are added to the CometBFT removal cache to aid with CometBFT
    /// mempool maintenance.
    #[instrument(skip_all)]
    pub(crate) async fn run_maintenance<F, O>(
        &self,
        current_account_nonce_getter: F,
    ) -> anyhow::Result<()>
    where
        F: Fn([u8; 20]) -> O,
        O: Future<Output = anyhow::Result<u32>>,
    {
        // grab needed main locks in normal order
        let mut all = self.all.write().await;
        let mut pending = self.pending.write().await;
        let mut parked = self.parked.write().await;

        // clean accounts of stale and expired tranasctions
        let mut removed_txs = pending
            .clean_accounts(&current_account_nonce_getter)
            .await
            .expect("failed to clean pending");
        removed_txs.append(
            &mut parked
                .clean_accounts(&current_account_nonce_getter)
                .await
                .expect("failed to clean pending"),
        );

        // run promotion logic in case transactions not in this mempool advanced account state
        let promotable_txs = parked
            .find_promotables(&current_account_nonce_getter)
            .await
            .expect("should work?");
        for ttx in promotable_txs {
            let current_account_nonce = current_account_nonce_getter(*ttx.address())
                .await
                .expect("failed to get account nonce for promotions");
            assert!(
                pending.add(ttx, current_account_nonce).is_ok(),
                "promotions should work"
            );
        }

        // remove from tracked and add to removal cache for cometbft
        let mut removal_cache = self.comet_bft_removal_cache.write().await;
        for (tx, reason) in removed_txs {
            all.remove(&tx.tx_hash());
            removal_cache.add(tx.tx_hash(), reason);
        }

        Ok(())
    }

    /// Returns the highest pending nonce for the given address if it exists in the mempool. Note:
    /// does not take into account gapped nonces in the parked queue. For example, if the
    /// pending queue for an account has nonces [0,1] and the parked queue has [3], [1] will be
    /// returned.
    #[instrument(skip_all)]
    pub(crate) async fn pending_nonce(&self, address: [u8; 20]) -> Option<u32> {
        self.pending.read().await.pending_nonce(address)
    }
}

#[cfg(test)]
mod test {
    use astria_core::crypto::SigningKey;

    use super::*;
    use crate::app::test_utils::get_mock_tx_parameterized;

    #[tokio::test]
    async fn insert() {
        let mempool = Mempool::new();
        let signing_key = SigningKey::from([1; 32]);

        // sign and insert nonce 1
        let tx1 = get_mock_tx_parameterized(1, &signing_key, [0; 32]);
        assert!(
            mempool.insert(tx1.clone(), 0).await.is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );

        // try to insert again
        assert!(
            matches!(
                mempool.insert(tx1.clone(), 0).await,
                Err(InsertionError::AlreadyPresent)
            ),
            "already present"
        );

        // try to replace nonce
        let tx1_replacement = get_mock_tx_parameterized(1, &signing_key, [1; 32]);
        assert!(
            matches!(
                mempool.insert(tx1_replacement.clone(), 0).await,
                Err(InsertionError::NonceTaken)
            ),
            "nonce replace not allowed"
        );

        // add too low nonce
        let tx0 = get_mock_tx_parameterized(0, &signing_key, [1; 32]);
        assert!(
            matches!(
                mempool.insert(tx0.clone(), 1).await,
                Err(InsertionError::NonceTooLow)
            ),
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

        // add nonces in odd order to trigger insertion promotion logic
        // sign and insert nonce 1
        let tx1 = get_mock_tx_parameterized(1, &signing_key, [0; 32]);
        assert!(
            mempool.insert(tx1.clone(), 0).await.is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );

        // sign and insert nonce 2
        let tx2 = get_mock_tx_parameterized(2, &signing_key, [0; 32]);
        assert!(
            mempool.insert(tx2.clone(), 0).await.is_ok(),
            "should be able to insert nonce 2 transaction into mempool"
        );

        // sign and insert nonce 0
        let tx0 = get_mock_tx_parameterized(0, &signing_key, [0; 32]);
        assert!(
            mempool.insert(tx0.clone(), 0).await.is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );

        // sign and insert nonce 4
        let tx4 = get_mock_tx_parameterized(4, &signing_key, [0; 32]);
        assert!(
            mempool.insert(tx4.clone(), 0).await.is_ok(),
            "should be able to insert nonce 4 transaction into mempool"
        );

        // assert size
        assert_eq!(mempool.len().await, 4);

        // mock nonce getter with nonce at 1
        let current_account_nonce_getter = |address: [u8; 20]| async move {
            if address == signing_address {
                return Ok(1);
            }
            Err(anyhow::anyhow!("invalid address"))
        };

        // grab building queue, should return transactions [1,2] since [0] was below and [4] is
        // gapped
        let mut builder_queue = mempool
            .builder_queue(current_account_nonce_getter)
            .await
            .expect("failed to get builder queue");

        // see contains first two transactions that should be pending
        let (returned_tx, _) = builder_queue
            .pop()
            .expect("should return lowest nonced transaction");
        assert_eq!(returned_tx.signed_tx().nonce(), 1, "nonce should be one");

        let (returned_tx, _) = builder_queue
            .pop()
            .expect("should return other transaction");
        assert_eq!(returned_tx.signed_tx().nonce(), 2, "nonce should be two");

        // see mempool's transactions just cloned, not consumed
        assert_eq!(mempool.len().await, 4);

        // run maintenance with simulated nonce to remove the nonces 0,1,2 and promote 4 from parked
        // to pending
        let current_account_nonce_getter = |address: [u8; 20]| async move {
            if address == signing_address {
                return Ok(4);
            }
            Err(anyhow::anyhow!("invalid address"))
        };
        mempool
            .run_maintenance(current_account_nonce_getter)
            .await
            .expect("failed to run maintenance");

        // assert mempool at 1
        assert_eq!(mempool.len().await, 1);

        // see transaction [4] properly promoted
        let mut builder_queue = mempool
            .builder_queue(current_account_nonce_getter)
            .await
            .expect("failed to get builder queue");
        let (returned_tx, _) = builder_queue.pop().expect("should return last transaction");
        assert_eq!(returned_tx.signed_tx().nonce(), 4, "nonce should be four");
    }

    #[tokio::test]
    #[allow(unused_variables)] // for matches! macro
    #[allow(clippy::too_many_lines)]
    async fn remove_invalid() {
        let mempool = Mempool::new();
        let signing_key = SigningKey::from([1; 32]);

        // sign and insert nonces 0,1 and 3,4,5
        let tx0 = get_mock_tx_parameterized(0, &signing_key, [0; 32]);
        assert!(
            mempool.insert(tx0.clone(), 0).await.is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );
        let tx1 = get_mock_tx_parameterized(1, &signing_key, [0; 32]);
        assert!(
            mempool.insert(tx1.clone(), 0).await.is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );
        let tx3 = get_mock_tx_parameterized(3, &signing_key, [0; 32]);
        assert!(
            mempool.insert(tx3.clone(), 0).await.is_ok(),
            "should be able to insert nonce 3 transaction into mempool"
        );
        let tx4 = get_mock_tx_parameterized(4, &signing_key, [0; 32]);
        assert!(
            mempool.insert(tx4.clone(), 0).await.is_ok(),
            "should be able to insert nonce 4 transaction into mempool"
        );
        let tx5 = get_mock_tx_parameterized(5, &signing_key, [0; 32]);
        assert!(
            mempool.insert(tx5.clone(), 0).await.is_ok(),
            "should be able to insert nonce 5 transaction into mempool"
        );
        assert_eq!(mempool.len().await, 5);

        let removal_reason = RemovalReason::FailedPrepareProposal("reason".to_string());

        // remove 4, should remove 4 and 5
        let mut removed_txs = mempool
            .remove_tx_invalid(&tx4, removal_reason.clone())
            .await;

        assert_eq!(
            removed_txs
                .pop()
                .expect("should return transaction")
                .signed_tx()
                .nonce(),
            4
        );
        assert_eq!(
            removed_txs
                .pop()
                .expect("should return transaction")
                .signed_tx()
                .nonce(),
            5
        );
        assert_eq!(mempool.len().await, 3);

        // remove 4 again is also ok
        let removed_txs = mempool
            .remove_tx_invalid(
                &tx4,
                RemovalReason::NonceStale, // shouldn't be inserted into removal cache
            )
            .await;
        assert_eq!(removed_txs.len(), 0);
        assert_eq!(mempool.len().await, 3);

        // remove 1, should remove 1 and 3
        let mut removed_txs = mempool
            .remove_tx_invalid(&tx1, removal_reason.clone())
            .await;

        assert_eq!(
            removed_txs
                .pop()
                .expect("should return transaction")
                .signed_tx()
                .nonce(),
            3
        );
        assert_eq!(
            removed_txs
                .pop()
                .expect("should return transaction")
                .signed_tx()
                .nonce(),
            1
        );
        assert_eq!(mempool.len().await, 1);

        // remove 0
        let mut removed_txs = mempool
            .remove_tx_invalid(&tx0, removal_reason.clone())
            .await;
        assert_eq!(
            removed_txs
                .pop()
                .expect("should return transaction")
                .signed_tx()
                .nonce(),
            0
        );
        assert_eq!(mempool.len().await, 0);

        // assert that all were added to the cometbft removal cache
        // and the expected reasons were tracked
        assert!(matches!(
            mempool
                .check_removed_comet_bft(tx0.sha256_of_proto_encoding())
                .await,
            removal_reason
        ));
        assert!(matches!(
            mempool
                .check_removed_comet_bft(tx1.sha256_of_proto_encoding())
                .await,
            removal_reason
        ));
        assert!(matches!(
            mempool
                .check_removed_comet_bft(tx3.sha256_of_proto_encoding())
                .await,
            Some(RemovalReason::LowerNonceInvalidated)
        ));
        assert!(matches!(
            mempool
                .check_removed_comet_bft(tx4.sha256_of_proto_encoding())
                .await,
            removal_reason
        ));
        assert!(matches!(
            mempool
                .check_removed_comet_bft(tx5.sha256_of_proto_encoding())
                .await,
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

        // sign and insert nonces 0,1
        let tx0 = get_mock_tx_parameterized(0, &signing_key_0, [0; 32]);
        assert!(
            mempool.insert(tx0.clone(), 0).await.is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );
        let tx1 = get_mock_tx_parameterized(1, &signing_key_0, [0; 32]);
        assert!(
            mempool.insert(tx1.clone(), 0).await.is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );

        // sign and insert nonces 100, 101
        let tx100 = get_mock_tx_parameterized(100, &signing_key_1, [0; 32]);
        assert!(
            mempool.insert(tx100.clone(), 100).await.is_ok(),
            "should be able to insert nonce 100 transaction into mempool"
        );
        let tx101 = get_mock_tx_parameterized(101, &signing_key_1, [0; 32]);
        assert!(
            mempool.insert(tx101.clone(), 100).await.is_ok(),
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
