#[cfg(feature = "benchmark")]
mod benchmarks;
mod mempool_inner;
mod mempool_state;
pub(crate) mod query;
mod transactions_container;

use std::{
    collections::HashMap,
    sync::Arc,
};

use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        TransactionId,
    },
    protocol::transaction::v1::{
        Transaction,
        TransactionStatus,
    },
};
use astria_eyre::eyre::Result;
use mempool_inner::MempoolInner;
pub(crate) use mempool_inner::RemovalReason;
#[cfg(feature = "benchmark")]
pub(crate) use mempool_inner::REMOVAL_CACHE_SIZE;
pub(crate) use mempool_state::get_account_balances;
use tokio::sync::RwLock;
use tracing::instrument;
pub(crate) use transactions_container::InsertionError;

use crate::{
    accounts,
    Metrics,
};

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
    inner: Arc<RwLock<MempoolInner>>,
}

impl Mempool {
    pub(crate) fn new(metrics: &'static Metrics, parked_max_tx_count: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(MempoolInner::new(metrics, parked_max_tx_count))),
        }
    }

    /// Returns a copy of all transactions and their hashes ready for execution, sorted first by the
    /// difference between a transaction and the account's current nonce and then by the time that
    /// the transaction was first seen by the appside mempool.
    pub(crate) async fn builder_queue(&self) -> Vec<([u8; 32], Arc<Transaction>)> {
        self.inner.read().await.pending.builder_queue()
    }

    /// Checks if a transaction was flagged to be removed from the `CometBFT` mempool. Will
    /// remove the transaction from the cache if it is present.
    #[instrument(skip_all, fields(tx_hash = %hex::encode(tx_hash)))]
    pub(crate) async fn check_removed_comet_bft(&self, tx_hash: [u8; 32]) -> Option<RemovalReason> {
        self.inner
            .write()
            .await
            .comet_bft_removal_cache
            .remove(tx_hash)
    }

    /// Returns true if the transaction is tracked as inserted.
    #[instrument(skip_all, fields(tx_hash = %hex::encode(tx_hash)))]
    pub(crate) async fn is_tracked(&self, tx_hash: [u8; 32]) -> bool {
        self.inner.read().await.contained_txs.contains(&tx_hash)
    }

    /// Returns the highest pending nonce for the given address if it exists in the mempool. Note:
    /// does not take into account gapped nonces in the parked queue. For example, if the
    /// pending queue for an account has nonces [0,1] and the parked queue has [3], [1] will be
    /// returned.
    #[instrument(skip_all, fields(address = %telemetry::display::base64(address)))]
    pub(crate) async fn pending_nonce(&self, address: &[u8; 20]) -> Option<u32> {
        self.inner.read().await.pending.pending_nonce(address)
    }

    /// Returns the number of transactions in the mempool.
    #[must_use]
    #[instrument(skip_all)]
    pub(crate) async fn len(&self) -> usize {
        self.inner.read().await.contained_txs.len()
    }

    /// Inserts a transaction into the mempool and does not allow for transaction replacement.
    /// Will return the reason for insertion failure if failure occurs.
    #[instrument(skip_all)]
    pub(crate) async fn insert(
        &self,
        tx: Arc<Transaction>,
        current_account_nonce: u32,
        current_account_balances: HashMap<IbcPrefixed, u128>,
        transaction_cost: HashMap<IbcPrefixed, u128>,
    ) -> Result<(), InsertionError> {
        self.inner
            .write()
            .await
            .insert(
                tx,
                current_account_nonce,
                current_account_balances,
                transaction_cost,
            )
            .await
    }

    /// Removes the target transaction and all transactions for associated account with higher
    /// nonces.
    ///
    /// This function should only be used to remove invalid/failing transactions and not executed
    /// transactions. Executed transactions will be removed in the `run_maintenance()` function.
    #[instrument(skip_all)]
    pub(crate) async fn remove_tx_invalid(
        &self,
        signed_tx: Arc<Transaction>,
        reason: RemovalReason,
    ) {
        self.inner
            .write()
            .await
            .remove_tx_invalid(signed_tx, reason)
            .await;
    }

    /// Updates stored transactions to reflect current blockchain state. Will remove transactions
    /// that have stale nonces or are expired
    #[instrument(skip_all)]
    pub(crate) async fn run_maintenance<S: accounts::StateReadExt>(&self, state: &S, recost: bool) {
        self.inner
            .write()
            .await
            .run_maintenance(state, recost)
            .await;
    }

    /// Returns a given transaction's status, as well as an optional reason for removal if the
    /// transaction is in the CometBFT removal cache.
    #[instrument(skip_all)]
    pub(crate) async fn get_transaction_status(
        &self,
        tx_id: &TransactionId,
    ) -> (TransactionStatus, Option<String>) {
        self.inner.read().await.get_transaction_status(tx_id)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

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
        mempool::mempool_inner::RemovalCache,
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
                .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );
        assert_eq!(mempool.len().await, 1);

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
        let tx1_replacement = MockTxBuilder::new()
            .nonce(1)
            .chain_id("test-chain-id")
            .build();
        assert_eq!(
            mempool
                .insert(
                    tx1_replacement.clone(),
                    0,
                    account_balances.clone(),
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
                .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );

        // sign and insert nonce 2
        let tx2 = MockTxBuilder::new().nonce(2).build();
        assert!(
            mempool
                .insert(tx2.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 2 transaction into mempool"
        );

        // sign and insert nonce 0
        let tx0 = MockTxBuilder::new().nonce(0).build();
        assert!(
            mempool
                .insert(tx0.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );

        // sign and insert nonce 4
        let tx4 = MockTxBuilder::new().nonce(4).build();
        assert!(
            mempool
                .insert(tx4.clone(), 0, account_balances.clone(), tx_cost.clone())
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

        mempool.run_maintenance(&mock_state, false).await;

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

        mempool.run_maintenance(&mock_state, false).await;

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

        mempool.run_maintenance(&mock_state, false).await;

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

        mempool.run_maintenance(&mock_state, false).await;

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
                .insert(tx0.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );
        let tx1 = MockTxBuilder::new().nonce(1).build();
        assert!(
            mempool
                .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 1 transaction into mempool"
        );
        let tx3 = MockTxBuilder::new().nonce(3).build();
        assert!(
            mempool
                .insert(tx3.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 3 transaction into mempool"
        );
        let tx4 = MockTxBuilder::new().nonce(4).build();
        assert!(
            mempool
                .insert(tx4.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 4 transaction into mempool"
        );
        let tx5 = MockTxBuilder::new().nonce(5).build();
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
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);

        // sign and insert nonces 0,1
        let tx0 = MockTxBuilder::new().nonce(0).build();
        assert!(
            mempool
                .insert(tx0.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .is_ok(),
            "should be able to insert nonce 0 transaction into mempool"
        );
        let tx1 = MockTxBuilder::new().nonce(1).build();
        assert!(
            mempool
                .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
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
                    account_balances.clone(),
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
                    account_balances.clone(),
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
            .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        assert!(mempool.is_tracked(tx1.id().get()).await);

        // check that the pending transaction is in the tracked set
        mempool
            .insert(tx0.clone(), 0, account_balances.clone(), tx_cost.clone())
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
            .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx0.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // remove the transacitons from the mempool via maintenance
        let mut mock_state = mock_state_getter().await;
        mock_state_put_account_nonce(
            &mut mock_state,
            astria_address_from_hex_string(ALICE_ADDRESS).as_bytes(),
            2,
        );
        mempool.run_maintenance(&mock_state, false).await;

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
            .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        mempool
            .insert(tx0.clone(), 0, account_balances.clone(), tx_cost.clone())
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
            .insert(tx0.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
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
            .insert(tx1.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        // size limit fails as expected
        assert_eq!(
            mempool
                .insert(tx0.clone(), 0, account_balances.clone(), tx_cost.clone())
                .await
                .unwrap_err(),
            InsertionError::ParkedSizeLimit,
            "size limit should be enforced"
        );
    }

    #[tokio::test]
    async fn get_transaction_status_pending_works_as_expected() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 10);

        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);
        let tx = MockTxBuilder::new().nonce(0).build();
        mempool
            .insert(tx.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        assert_eq!(mempool.len().await, 1);
        assert_eq!(
            mempool.get_transaction_status(&tx.id()).await,
            (TransactionStatus::Pending, None)
        );
    }

    #[tokio::test]
    async fn get_transaction_status_parked_works_as_expected() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 10);

        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);
        let tx = MockTxBuilder::new().nonce(1).build(); // The nonce gap (1 vs. 0) will park the transaction
        mempool
            .insert(tx.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        assert_eq!(mempool.len().await, 1);
        assert_eq!(
            mempool.get_transaction_status(&tx.id()).await,
            (TransactionStatus::Parked, None)
        );
    }

    #[tokio::test]
    async fn get_transaction_status_not_found_works_as_expected() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 10);

        assert_eq!(mempool.len().await, 0);
        assert_eq!(
            mempool
                .get_transaction_status(&TransactionId::new([0u8; 32]))
                .await,
            (TransactionStatus::NotFound, None)
        );
    }

    #[tokio::test]
    async fn get_transaction_status_removal_cache_works_as_expected() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 10);

        let account_balances = mock_balances(100, 100);
        let tx_cost = mock_tx_cost(10, 10, 0);
        let tx = MockTxBuilder::new().nonce(0).build();
        mempool
            .insert(tx.clone(), 0, account_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        let reason = RemovalReason::FailedPrepareProposal("test".to_string());
        mempool.remove_tx_invalid(tx.clone(), reason.clone()).await;
        assert_eq!(
            mempool.get_transaction_status(&tx.id()).await,
            (TransactionStatus::RemovalCache, Some(reason.to_string()))
        );
    }
}
