use std::collections::{
    HashMap,
    VecDeque,
};

use astria_core::primitive::v1::TransactionId;
use tendermint::abci::types::ExecTxResult;
use tokio::time::Instant;

/// The maximum number of transactions that can be stored in the [`RecentlyIncludedTransactions`]
/// cache. This number is calculated based on an assumed timeout time of 1 minute and a maximum
/// throughput of 1000 transactions per second.
const MAX_RECENTLY_INCLUDED_TRANSACTIONS: usize = 60_000;
/// The timeout time for the transactions in the [`RecentlyIncludedTransactions`] cache.
const MAX_TIME_RECENTLY_INCLUDED_TRANSACTIONS_SECS: u64 = 60;

pub(super) struct IncludedTransactionMetadata {
    height: u64,
    result: ExecTxResult,
}

impl IncludedTransactionMetadata {
    pub(super) fn height(&self) -> u64 {
        self.height
    }

    pub(super) fn result(&self) -> &ExecTxResult {
        &self.result
    }
}

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub(super) enum IncludedTransactionError {
    #[error("Transaction ID {0} already exists in the cache")]
    IdAlreadyExists(TransactionId),
    #[error("Transaction insertion failed due to an internal error")]
    InsertionInternalError,
}

/// A cache of transaction data for transactions which were recently included in a block. The cache
/// has a maximum size of [`MAX_RECENTLY_INCLUDED_TRANSACTIONS`] transactions. Transactions are
/// cleared from the cache after [`MAX_TIME_RECENTLY_INCLUDED_TRANSACTIONS_SECS`] seconds if the
/// cache is not full.
///
/// Supports O(1) lookups by transaction ID while maintaining O(1) insertion and removal of oldest
/// element. Worst case time complexity for cleaning stale transactions is O(n), where n is the
/// number of transactions in the cache.
pub(super) struct RecentlyIncludedTransactions {
    /// Transaction IDs in chronological order (oldest first)
    timestamped_ids: VecDeque<(TransactionId, Instant)>,
    /// Hash map containing transaction metadata, keyed by transaction ID
    transactions: HashMap<TransactionId, IncludedTransactionMetadata>,
}

impl RecentlyIncludedTransactions {
    pub(super) fn new() -> Self {
        Self {
            timestamped_ids: VecDeque::new(),
            transactions: HashMap::new(),
        }
    }

    /// Looks up transaction metadata by its transaction ID, returning
    /// `Some(IncludedTransactionMetadata)` if found, or `None` if not found.
    pub(super) fn get_by_tx_id(
        &self,
        tx_id: &TransactionId,
    ) -> Option<&IncludedTransactionMetadata> {
        self.transactions.get(tx_id)
    }

    /// Adds new transaction data to the cache. Cleans stale transactions before attempting to add
    /// the new transaction. If the cache is full, removes the oldest transaction until the cache
    /// is not full. Returns an error if the transaction ID already exists in the cache.
    pub(super) fn add_transaction(
        &mut self,
        tx_id: TransactionId,
        height: u64,
        result: ExecTxResult,
    ) -> Result<(), IncludedTransactionError> {
        // If the cache is full, remove the oldest transaction until the cache is not full.
        while self.timestamped_ids.len() >= MAX_RECENTLY_INCLUDED_TRANSACTIONS {
            let (oldest_tx_id, _) = self
                .timestamped_ids
                .pop_front()
                .ok_or(IncludedTransactionError::InsertionInternalError)?;
            self.transactions.remove(&oldest_tx_id);
        }

        // Check if the transaction ID already exists in the cache.
        if self.transactions.contains_key(&tx_id) {
            return Err(IncludedTransactionError::IdAlreadyExists(tx_id));
        }

        // Clean any stale transactions before adding a new one.
        self.clean_stale();

        // Add new transaction to the cache and push to the back of the ID vec.
        self.timestamped_ids.push_back((tx_id, Instant::now()));
        self.transactions.insert(
            tx_id,
            IncludedTransactionMetadata {
                height,
                result,
            },
        );
        Ok(())
    }

    /// Removes all transactions from the cache that are older than
    /// [`MAX_TIME_RECENTLY_INCLUDED_TRANSACTIONS_SECS`] seconds.
    pub(super) fn clean_stale(&mut self) {
        let now = Instant::now();
        let mut split_index = 0;

        while split_index < self.transactions.len() {
            let (tx_id, timestamp) = &self.timestamped_ids[split_index];
            if now.duration_since(*timestamp).as_secs()
                < MAX_TIME_RECENTLY_INCLUDED_TRANSACTIONS_SECS
            {
                break;
            }
            // Remove the transaction from the hash map
            self.transactions.remove(tx_id);
            split_index = split_index.saturating_add(1);
        }

        // Remove from the vector
        self.timestamped_ids.drain(0..split_index);
    }
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::TRANSACTION_ID_LEN;
    use tokio::time;

    use super::*;

    // Helper function for creating unique transaction IDs for all seeds in the range of
    // 0..u64::MAX.
    fn dummy_tx_id(seed: u64) -> TransactionId {
        let bytes = seed.to_be_bytes();
        let mut destination = [0u8; TRANSACTION_ID_LEN];

        destination[..8].copy_from_slice(&bytes[..8]);
        TransactionId::new(destination)
    }

    #[test]
    fn construct_add_and_get_work_as_expected() {
        let mut cache = RecentlyIncludedTransactions::new();

        let tx_id_1 = dummy_tx_id(0);
        let tx_height_1 = 1;
        let tx_result_1 = ExecTxResult::default();

        cache
            .add_transaction(tx_id_1, tx_height_1, tx_result_1.clone())
            .unwrap();
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.transactions.len(), 1);
        let transaction_1 = cache.get_by_tx_id(&cache.timestamped_ids[0].0).unwrap();
        assert_eq!(transaction_1.height, tx_height_1);
        assert_eq!(transaction_1.result, tx_result_1);

        let tx_id_2 = dummy_tx_id(1);
        let tx_height_2 = 2;
        let tx_result_2 = ExecTxResult {
            log: "transaction 2".to_string(),
            ..ExecTxResult::default()
        };
        assert_ne!(tx_id_1, tx_id_2);
        assert_ne!(tx_height_1, tx_height_2);
        assert_ne!(tx_result_1, tx_result_2);

        cache
            .add_transaction(tx_id_2, tx_height_2, tx_result_2.clone())
            .unwrap();
        assert_eq!(cache.timestamped_ids.len(), 2);
        assert_eq!(cache.transactions.len(), 2);
        let transaction_2 = cache.get_by_tx_id(&cache.timestamped_ids[1].0).unwrap();
        assert_eq!(transaction_2.height, tx_height_2);
        assert_eq!(transaction_2.result, tx_result_2);

        // Test lookup of non-existent tx
        let non_existent_tx_id = dummy_tx_id(99);
        assert_ne!(tx_id_1, non_existent_tx_id);
        assert_ne!(tx_id_2, non_existent_tx_id);
        assert!(cache.get_by_tx_id(&non_existent_tx_id).is_none());
    }

    #[tokio::test]
    async fn clean_stale_works_as_expected() {
        let mut cache = RecentlyIncludedTransactions::new();
        let tx_id_1 = dummy_tx_id(0);
        let tx_height_1 = 1;
        let tx_result_1 = ExecTxResult::default();

        cache
            .add_transaction(tx_id_1, tx_height_1, tx_result_1.clone())
            .unwrap();
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.transactions.len(), 1);
        assert!(cache.get_by_tx_id(&tx_id_1).is_some());

        time::pause();
        time::advance(time::Duration::from_secs(61)).await;
        cache.clean_stale();
        assert_eq!(cache.timestamped_ids.len(), 0);
        assert_eq!(cache.transactions.len(), 0);
        assert!(cache.get_by_tx_id(&tx_id_1).is_none());
    }

    #[tokio::test]
    async fn add_transaction_clears_stale() {
        let mut cache = RecentlyIncludedTransactions::new();
        let tx_id_for_removal = dummy_tx_id(0);
        let tx_height_for_removal = 1;
        let tx_result_for_removal = ExecTxResult::default();

        cache
            .add_transaction(
                tx_id_for_removal,
                tx_height_for_removal,
                tx_result_for_removal.clone(),
            )
            .unwrap();
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.transactions.len(), 1);
        let transaction_for_removal = cache.get_by_tx_id(&cache.timestamped_ids[0].0).unwrap();
        assert_eq!(transaction_for_removal.height, tx_height_for_removal);
        assert_eq!(transaction_for_removal.result, tx_result_for_removal);

        time::pause();
        time::advance(time::Duration::from_secs(61)).await;

        let tx_id_2 = dummy_tx_id(1);
        let tx_height_2 = 2;
        let tx_result_2 = ExecTxResult {
            log: "transaction 2".to_string(),
            ..ExecTxResult::default()
        };

        // Should remove the old transaction and add the new one
        cache
            .add_transaction(tx_id_2, tx_height_2, tx_result_2.clone())
            .unwrap();

        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.transactions.len(), 1);
        let transaction_2 = cache.get_by_tx_id(&cache.timestamped_ids[0].0).unwrap();
        assert_eq!(transaction_2.height, tx_height_2);
        assert_eq!(transaction_2.result, tx_result_2);
    }

    #[test]
    fn add_transaction_removes_oldest_when_full() {
        let mut cache = RecentlyIncludedTransactions::new();
        for i in 0..MAX_RECENTLY_INCLUDED_TRANSACTIONS {
            let tx_id = dummy_tx_id(i as u64);
            let tx_height = i as u64;
            let tx_result = ExecTxResult {
                log: format!("transaction {i}"),
                ..ExecTxResult::default()
            };
            cache.add_transaction(tx_id, tx_height, tx_result).unwrap();
        }

        // Check that the cache is full and that first transaction is the oldest one.
        assert_eq!(cache.transactions.len(), MAX_RECENTLY_INCLUDED_TRANSACTIONS);
        assert_eq!(
            cache.timestamped_ids.len(),
            MAX_RECENTLY_INCLUDED_TRANSACTIONS
        );
        let pre_clean_first_tx = cache.get_by_tx_id(&cache.timestamped_ids[0].0).unwrap();
        assert_eq!(pre_clean_first_tx.height, 0);
        assert_eq!(
            pre_clean_first_tx.result,
            ExecTxResult {
                log: "transaction 0".to_string(),
                ..ExecTxResult::default()
            }
        );

        // Add one more transaction to trigger the removal of the oldest transaction.
        let tx_id_to_add = dummy_tx_id(0);
        let tx_height_to_add = 1_234_567_890;
        let tx_result_to_add = ExecTxResult {
            log: "ethan_was here".to_string(),
            ..ExecTxResult::default()
        };
        cache
            .add_transaction(tx_id_to_add, tx_height_to_add, tx_result_to_add.clone())
            .unwrap();

        // Check that cache is still full and that the oldest transaction was removed.
        assert_eq!(cache.transactions.len(), MAX_RECENTLY_INCLUDED_TRANSACTIONS);
        assert_eq!(
            cache.timestamped_ids.len(),
            MAX_RECENTLY_INCLUDED_TRANSACTIONS
        );
        let post_clean_first_tx = cache.get_by_tx_id(&cache.timestamped_ids[0].0).unwrap();
        assert_eq!(post_clean_first_tx.height, 1);
        assert_eq!(
            post_clean_first_tx.result,
            ExecTxResult {
                log: "transaction 1".to_string(),
                ..ExecTxResult::default()
            }
        );

        // If removal failed, this should grab the first transaction in the cache since they share
        // the same ID.
        let post_clean_last_tx = cache
            .get_by_tx_id(&cache.timestamped_ids[MAX_RECENTLY_INCLUDED_TRANSACTIONS - 1].0)
            .unwrap();

        // Check that the last transaction is the one we just added.
        assert_eq!(post_clean_last_tx.height, tx_height_to_add);
        assert_eq!(post_clean_last_tx.result, tx_result_to_add);
    }

    #[test]
    fn add_duplicate_transaction_returns_error() {
        let mut cache = RecentlyIncludedTransactions::new();
        let tx_id = dummy_tx_id(0);
        let tx_height = 1;
        let tx_result = ExecTxResult::default();

        // First addition should succeed
        let result = cache.add_transaction(tx_id, tx_height, tx_result.clone());
        assert!(result.is_ok());
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.transactions.len(), 1);

        // Second addition with same tx_id should fail
        let err = cache
            .add_transaction(tx_id, tx_height + 1, tx_result.clone())
            .unwrap_err();
        assert_eq!(err, IncludedTransactionError::IdAlreadyExists(tx_id));

        // Cache state should remain unchanged
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.transactions.len(), 1);
    }

    #[tokio::test]
    async fn clean_stale_handles_mixed_ages() {
        let mut cache = RecentlyIncludedTransactions::new();

        // Add transactions with different timestamps
        time::pause();

        // Add old transaction
        let tx_id_old = dummy_tx_id(0);
        let tx_height_old = 1;
        let tx_result_old = ExecTxResult::default();
        cache
            .add_transaction(tx_id_old, tx_height_old, tx_result_old.clone())
            .unwrap();

        // Advance time to make sure both transactions aren't invalidated at the same time
        time::advance(time::Duration::from_secs(30)).await;

        // Add fresh transaction
        let tx_id_fresh = dummy_tx_id(1);
        let tx_height_fresh = 2;
        let tx_result_fresh = ExecTxResult {
            log: "fresh transaction".to_string(),
            ..ExecTxResult::default()
        };
        cache
            .add_transaction(tx_id_fresh, tx_height_fresh, tx_result_fresh.clone())
            .unwrap();

        // Advance time to make sure the old transaction is stale
        time::advance(time::Duration::from_secs(31)).await;

        // Clean stale entries
        cache.clean_stale();

        // Check that only the old transaction was removed
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.transactions.len(), 1);
        assert!(cache.get_by_tx_id(&tx_id_old).is_none());
        assert!(cache.get_by_tx_id(&tx_id_fresh).is_some());

        let remaining_tx = cache.get_by_tx_id(&tx_id_fresh).unwrap();
        assert_eq!(remaining_tx.height, tx_height_fresh);
        assert_eq!(remaining_tx.result, tx_result_fresh);
    }

    #[tokio::test]
    async fn clean_stale_with_empty_cache() {
        let mut cache = RecentlyIncludedTransactions::new();

        assert_eq!(cache.timestamped_ids.len(), 0);
        assert_eq!(cache.transactions.len(), 0);

        // Should not do anything
        cache.clean_stale();

        assert_eq!(cache.timestamped_ids.len(), 0);
        assert_eq!(cache.transactions.len(), 0);
    }

    #[test]
    fn transaction_metadata_accessors() {
        let height = 42;
        let result = ExecTxResult {
            log: "test result".to_string(),
            ..ExecTxResult::default()
        };

        let metadata = IncludedTransactionMetadata {
            height,
            result: result.clone(),
        };

        assert_eq!(metadata.height(), height);
        assert_eq!(metadata.result(), &result);
    }
}
