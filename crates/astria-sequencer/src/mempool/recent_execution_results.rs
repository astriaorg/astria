use std::{
    collections::{
        HashMap,
        VecDeque,
    },
    sync::Arc,
    time::Duration,
};

use astria_core::primitive::v1::TransactionId;
use tendermint::abci::types::ExecTxResult;
use tokio::time::Instant;
use tracing::{
    instrument,
    warn,
};

/// The timeout time for the results in the [`RecentExecutionResults`] cache.
const RETENTION_DURATION: Duration = Duration::from_secs(60);

#[derive(Debug)]
#[cfg_attr(feature = "benchmark", derive(Clone))]
pub(super) struct ExecutionResult {
    block_height: u64,
    result: Arc<ExecTxResult>,
}

impl ExecutionResult {
    pub(super) fn block_height(&self) -> u64 {
        self.block_height
    }

    pub(super) fn result(&self) -> Arc<ExecTxResult> {
        self.result.clone()
    }
}

/// A cache of execution results for transactions which were recently included in a block. The cache
/// has a maximum size of [`MAX_RECENT_EXECUTION_RESULTS`] results. Results are cleared from the
/// cache after [`RETENTION_DURATION`] if the cache is not full.
///
/// Supports O(1) lookups by transaction ID while maintaining O(1) insertion and removal of oldest
/// element. Worst case time complexity for cleaning stale transactions is O(n), where n is the
/// number of results in the cache.
#[derive(Debug)]
#[cfg_attr(feature = "benchmark", derive(Clone))]
pub(super) struct RecentExecutionResults {
    /// The maximum number of results that can be stored in the cache.
    max_size: usize,
    /// Transaction IDs in chronological order (oldest first)
    timestamped_ids: VecDeque<(TransactionId, Instant)>,
    /// Hash map containing transaction execution results, keyed by transaction ID
    execution_results: HashMap<TransactionId, ExecutionResult>,
}

impl RecentExecutionResults {
    pub(super) fn new(max_size: usize) -> Self {
        Self {
            max_size,
            timestamped_ids: VecDeque::new(),
            execution_results: HashMap::new(),
        }
    }

    /// Looks up an execution result by its correspondent transaction ID, returning
    /// `Some(ExecutionResult)` if found, or `None` if not found.
    pub(super) fn get(&self, tx_id: &TransactionId) -> Option<&ExecutionResult> {
        self.execution_results.get(tx_id)
    }

    /// Returns the number of execution results in the cache.
    pub(super) fn len(&self) -> usize {
        self.execution_results.len()
    }

    /// Adds new execution results to the cache. Cleans stale results before attempting to add
    /// the new one. If the cache is full, removes the oldest execution result until the cache
    /// is not full. If the transaction ID already exists in the cache, it is not added again.
    #[instrument(skip_all)]
    pub(super) fn add(
        &mut self,
        execution_results: HashMap<TransactionId, Arc<ExecTxResult>>,
        block_height: u64,
    ) {
        let now = Instant::now();

        // Clean any stale results before adding a new one.
        self.clean_stale(now);

        for (tx_id, result) in execution_results {
            // If the cache is full, remove the oldest result until the cache is not full.
            while self.timestamped_ids.len() >= self.max_size {
                let Some((oldest_tx_id, _)) = self.timestamped_ids.pop_front() else {
                    warn!(
                        "failed to remove oldest execution result from recent execution results \
                         cache to make space for tx {tx_id}, not adding it"
                    );
                    return;
                };
                self.execution_results.remove(&oldest_tx_id);
            }

            // Add new result to the cache and push to the back of the ID vec.
            if self
                .execution_results
                .insert(
                    tx_id,
                    ExecutionResult {
                        block_height,
                        result,
                    },
                )
                .is_some()
            {
                warn!(
                    "transaction ID {tx_id} already exists in recent execution results cache, not \
                     adding it. this may indicate duplicate transaction execution"
                );
            } else {
                self.timestamped_ids.push_back((tx_id, now));
            }
        }
    }

    /// Removes all transactions from the cache that are older than
    /// [`RETENTION_DURATION`].
    #[instrument(skip_all)]
    pub(super) fn clean_stale(&mut self, now: Instant) {
        let partition_index = self
            .timestamped_ids
            .partition_point(|(_, timestamp)| now.duration_since(*timestamp) > RETENTION_DURATION);
        for (tx_id, _) in self.timestamped_ids.drain(0..partition_index) {
            if self.execution_results.remove(&tx_id).is_none() {
                warn!(
                    "transaction ID {tx_id} not found in recent execution results cache, not \
                     removing it"
                );
            }
        }
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
        let mut cache = RecentExecutionResults::new(60_000);

        let tx_id_1 = dummy_tx_id(0);
        let tx_height_1 = 1;
        let tx_result_1 = ExecTxResult::default();

        cache.add(
            std::iter::once((tx_id_1, Arc::new(tx_result_1.clone()))).collect(),
            tx_height_1,
        );
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.execution_results.len(), 1);
        let transaction_1 = cache.get(&cache.timestamped_ids[0].0).unwrap();
        assert_eq!(transaction_1.block_height(), tx_height_1);
        assert_eq!(*transaction_1.result(), tx_result_1);

        let tx_id_2 = dummy_tx_id(1);
        let tx_height_2 = 2;
        let tx_result_2 = ExecTxResult {
            log: "transaction 2".to_string(),
            ..ExecTxResult::default()
        };
        assert_ne!(tx_id_1, tx_id_2);
        assert_ne!(tx_height_1, tx_height_2);
        assert_ne!(tx_result_1, tx_result_2);

        cache.add(
            std::iter::once((tx_id_2, Arc::new(tx_result_2.clone()))).collect(),
            tx_height_2,
        );
        assert_eq!(cache.timestamped_ids.len(), 2);
        assert_eq!(cache.execution_results.len(), 2);
        let transaction_2 = cache.get(&cache.timestamped_ids[1].0).unwrap();
        assert_eq!(transaction_2.block_height(), tx_height_2);
        assert_eq!(*transaction_2.result(), tx_result_2);

        // Test lookup of non-existent tx
        let non_existent_tx_id = dummy_tx_id(99);
        assert_ne!(tx_id_1, non_existent_tx_id);
        assert_ne!(tx_id_2, non_existent_tx_id);
        assert!(cache.get(&non_existent_tx_id).is_none());
    }

    #[tokio::test]
    async fn clean_stale_works_as_expected() {
        let mut cache = RecentExecutionResults::new(60_000);
        let tx_id_1 = dummy_tx_id(0);
        let tx_height_1 = 1;
        let tx_result_1 = ExecTxResult::default();

        cache.add(
            std::iter::once((tx_id_1, Arc::new(tx_result_1.clone()))).collect(),
            tx_height_1,
        );
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.execution_results.len(), 1);
        assert!(cache.get(&tx_id_1).is_some());

        time::pause();
        time::advance(time::Duration::from_secs(61)).await;
        cache.clean_stale(Instant::now());
        assert_eq!(cache.timestamped_ids.len(), 0);
        assert_eq!(cache.execution_results.len(), 0);
        assert!(cache.get(&tx_id_1).is_none());
    }

    #[tokio::test]
    async fn add_transaction_clears_stale() {
        let mut cache = RecentExecutionResults::new(60_000);
        let tx_id_for_removal = dummy_tx_id(0);
        let tx_height_for_removal = 1;
        let tx_result_for_removal = ExecTxResult::default();

        cache.add(
            std::iter::once((tx_id_for_removal, Arc::new(tx_result_for_removal.clone()))).collect(),
            tx_height_for_removal,
        );
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.execution_results.len(), 1);
        let transaction_for_removal = cache.get(&cache.timestamped_ids[0].0).unwrap();
        assert_eq!(
            transaction_for_removal.block_height(),
            tx_height_for_removal
        );
        assert_eq!(*transaction_for_removal.result(), tx_result_for_removal);

        time::pause();
        time::advance(time::Duration::from_secs(61)).await;

        let tx_id_2 = dummy_tx_id(1);
        let tx_height_2 = 2;
        let tx_result_2 = ExecTxResult {
            log: "transaction 2".to_string(),
            ..ExecTxResult::default()
        };

        // Should remove the old transaction and add the new one
        cache.add(
            std::iter::once((tx_id_2, Arc::new(tx_result_2.clone()))).collect(),
            tx_height_2,
        );

        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.execution_results.len(), 1);
        let transaction_2 = cache.get(&cache.timestamped_ids[0].0).unwrap();
        assert_eq!(transaction_2.block_height(), tx_height_2);
        assert_eq!(*transaction_2.result(), tx_result_2);
    }

    #[test]
    fn add_transaction_removes_oldest_when_full() {
        let max_size = 10;
        let mut cache = RecentExecutionResults::new(max_size);
        for i in 0..max_size {
            let tx_id = dummy_tx_id(i as u64);
            let tx_height = i as u64;
            let tx_result = ExecTxResult {
                log: format!("transaction {i}"),
                ..ExecTxResult::default()
            };
            cache.add(
                std::iter::once((tx_id, Arc::new(tx_result))).collect(),
                tx_height,
            );
        }

        // Check that the cache is full and that first transaction is the oldest one.
        assert_eq!(cache.execution_results.len(), max_size);
        assert_eq!(cache.timestamped_ids.len(), max_size);
        let pre_clean_first_tx = cache.get(&cache.timestamped_ids[0].0).unwrap();
        assert_eq!(pre_clean_first_tx.block_height(), 0);
        assert_eq!(
            *pre_clean_first_tx.result(),
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
        cache.add(
            std::iter::once((tx_id_to_add, Arc::new(tx_result_to_add.clone()))).collect(),
            tx_height_to_add,
        );

        // Check that cache is still full and that the oldest transaction was removed.
        assert_eq!(cache.execution_results.len(), max_size);
        assert_eq!(cache.timestamped_ids.len(), max_size);
        let post_clean_first_tx = cache.get(&cache.timestamped_ids[0].0).unwrap();
        assert_eq!(post_clean_first_tx.block_height(), 1);
        assert_eq!(
            *post_clean_first_tx.result(),
            ExecTxResult {
                log: "transaction 1".to_string(),
                ..ExecTxResult::default()
            }
        );

        // If removal failed, this should grab the first transaction in the cache since they share
        // the same ID.
        let post_clean_last_tx = cache.get(&cache.timestamped_ids[max_size - 1].0).unwrap();

        // Check that the last transaction is the one we just added.
        assert_eq!(post_clean_last_tx.block_height(), tx_height_to_add);
        assert_eq!(*post_clean_last_tx.result(), tx_result_to_add);
    }

    #[test]
    fn add_duplicate_transaction_is_noop() {
        let mut cache = RecentExecutionResults::new(60_000);
        let tx_id = dummy_tx_id(0);
        let tx_height = 1;
        let tx_result = ExecTxResult::default();

        // First addition should succeed
        cache.add(
            std::iter::once((tx_id, Arc::new(tx_result.clone()))).collect(),
            tx_height,
        );
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.execution_results.len(), 1);

        // Second addition with same tx_id should be noop
        cache.add(
            std::iter::once((tx_id, Arc::new(tx_result.clone()))).collect(),
            tx_height + 1,
        );

        // Cache state should remain unchanged
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.execution_results.len(), 1);
    }

    #[tokio::test]
    async fn clean_stale_handles_mixed_ages() {
        let mut cache = RecentExecutionResults::new(60_000);

        // Add transactions with different timestamps
        time::pause();

        // Add old transaction
        let tx_id_old = dummy_tx_id(0);
        let tx_height_old = 1;
        let tx_result_old = ExecTxResult::default();
        cache.add(
            std::iter::once((tx_id_old, Arc::new(tx_result_old.clone()))).collect(),
            tx_height_old,
        );

        // Advance time to make sure both transactions aren't invalidated at the same time
        time::advance(time::Duration::from_secs(30)).await;

        // Add fresh transaction
        let tx_id_fresh = dummy_tx_id(1);
        let tx_height_fresh = 2;
        let tx_result_fresh = ExecTxResult {
            log: "fresh transaction".to_string(),
            ..ExecTxResult::default()
        };
        cache.add(
            std::iter::once((tx_id_fresh, Arc::new(tx_result_fresh.clone()))).collect(),
            tx_height_fresh,
        );

        // Advance time to make sure the old transaction is stale
        time::advance(time::Duration::from_secs(31)).await;

        // Clean stale entries
        cache.clean_stale(Instant::now());

        // Check that only the old transaction was removed
        assert_eq!(cache.timestamped_ids.len(), 1);
        assert_eq!(cache.execution_results.len(), 1);
        assert!(cache.get(&tx_id_old).is_none());
        assert!(cache.get(&tx_id_fresh).is_some());

        let remaining_tx = cache.get(&tx_id_fresh).unwrap();
        assert_eq!(remaining_tx.block_height(), tx_height_fresh);
        assert_eq!(*remaining_tx.result(), tx_result_fresh);
    }

    #[tokio::test]
    async fn clean_stale_with_empty_cache() {
        let mut cache = RecentExecutionResults::new(60_000);

        assert_eq!(cache.timestamped_ids.len(), 0);
        assert_eq!(cache.execution_results.len(), 0);

        // Should not do anything
        cache.clean_stale(Instant::now());

        assert_eq!(cache.timestamped_ids.len(), 0);
        assert_eq!(cache.execution_results.len(), 0);
    }

    #[test]
    fn transaction_metadata_accessors() {
        let block_height = 42;
        let result = ExecTxResult {
            log: "test result".to_string(),
            ..ExecTxResult::default()
        };

        let metadata = ExecutionResult {
            block_height,
            result: Arc::new(result.clone()),
        };

        assert_eq!(metadata.block_height(), block_height);
        assert_eq!(*metadata.result(), result);
    }
}
