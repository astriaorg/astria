use std::collections::{
    BTreeMap,
    HashMap,
};

use color_eyre::eyre::Result;
use tendermint::{
    block::Height,
    hash::Hash,
};
use tracing::{
    debug,
    info,
};

use crate::types::SequencerBlockSubset;

/// A queue for the SequencerBlockSubset type that holds blocks that are
/// pending or not yet ready for execution.
///
/// This Queue handles all the fork choice logic for incoming Sequencer blocks. It is responsible
/// for determining which blocks are ready to pass on to execution. The queue will hold on to the
/// other data that may have been recieved out of order. Whenever a new block is recieved,
/// it checkes for continuity among the blocks that are currently in the queue or deletes that data
/// if it is no longer needed or becomes stale.
#[derive(Debug, Clone)]
pub(super) struct Queue {
    head_height: Height,
    most_recent_soft_hash: Hash,

    // The collection of all pending blocks. the blocks in this map at Height ==
    // Queue.head_height are the head blocks
    pending_blocks: HashMap<Height, HashMap<Hash, SequencerBlockSubset>>,
    // All blocks in order by height that can be considered safe because they
    // have a child block
    soft_blocks: BTreeMap<Height, SequencerBlockSubset>,
}

impl Queue {
    pub(super) fn new() -> Self {
        Self {
            head_height: Height::default(),
            most_recent_soft_hash: Hash::default(),
            pending_blocks: HashMap::new(),
            soft_blocks: BTreeMap::new(),
        }
    }

    /// Inserts a new block into the ExecutorQueue.
    ///
    /// This is the only way to add data to the queue. When inserting blocks,
    /// the internal state of the queue will also be updated to properly order
    /// and arrange all blocks in the queue, based on the Tendermint/CometBFT fork
    /// choice rules.
    pub(super) fn insert(&mut self, block: SequencerBlockSubset) -> Result<Option<Hash>> {
        // if the block is already in the queue, return its hash
        if self.is_block_present(&block) {
            debug!(
                block.height = %block.height(),
                block.hash = %block.block_hash(),
                "block is already present in the queue"
            );
            return Ok(None);
        }

        // if the block is stale, ignore it
        if block.header().height < self.head_height {
            debug!(
                block.height = %block.height(),
                "block is stale and will not be added to the queue"
            );
            return Ok(None);
        }

        // if the block is at the head height OR in the future, just add it to
        // the pending blocks
        self.insert_to_pending_blocks(block.clone());

        info!(
            block.height = %block.height(),
            block.hash = %block.block_hash(),
            "block added to queue"
        );
        Ok(Some(block.block_hash()))
    }

    /// Removes and returns all "soft" and "Head" blocks in the queue, inorder
    /// from oldest to newest.
    ///
    /// This function returns an `Option<Vec<SequencerBlockData>>`. A `Some`
    /// value contains a vector of `SequencerBlockData` that are ready to be
    /// passed on to execution.
    /// A `None` value indicates that there are no blocks in the queue that are
    /// ready to be passed on. A `None` value does not mean there are no blocks
    /// in the queue.
    pub(super) fn pop_blocks(&mut self) -> Option<Vec<SequencerBlockSubset>> {
        let mut output_blocks: Vec<SequencerBlockSubset> = vec![];

        let soft_blocks = self.pop_soft_blocks();
        if let Some(mut soft_blocks) = soft_blocks {
            output_blocks.append(soft_blocks.as_mut());
        }
        if let Some(mut head_blocks) = self.pop_head_blocks() {
            output_blocks.append(head_blocks.as_mut());
        }

        if !output_blocks.is_empty() {
            Some(output_blocks)
        } else {
            None
        }
    }

    /// Removes and returns all "soft" blocks in the queue, inorder from oldest
    /// to newest.
    ///
    /// This function returns an `Option<Vec<SequencerBlockData>>`. A `Some`
    /// value contains a vector of `SequencerBlockData` that are ready to be
    /// passed on to execution.
    /// A `None` value indicates that there are no blocks in the queue that are
    /// ready to be passed on. A `None` value does not mean there are no blocks
    /// in the queue.
    pub(super) fn pop_soft_blocks(&mut self) -> Option<Vec<SequencerBlockSubset>> {
        let mut returned_soft_blocks: Vec<SequencerBlockSubset> =
            self.soft_blocks.values().cloned().collect();
        if !returned_soft_blocks.is_empty() {
            returned_soft_blocks.sort();
            self.soft_blocks.clear();
            let highest_soft_block = returned_soft_blocks[returned_soft_blocks.len() - 1].clone();
            self.head_height = highest_soft_block.height().increment();
            self.remove_data_below_height(self.head_height);
            Some(returned_soft_blocks)
        } else {
            None
        }
    }

    /// Return all the blocks at the head height
    fn pop_head_blocks(&mut self) -> Option<Vec<SequencerBlockSubset>> {
        if let Some(head_blocks) = self.pending_blocks.get_mut(&self.head_height) {
            let mut output_blocks: Vec<SequencerBlockSubset> = vec![];
            let tmp_blocks = head_blocks.clone();
            let mut blocks: Vec<&SequencerBlockSubset> = tmp_blocks.values().collect();
            blocks.sort();
            for block in blocks {
                output_blocks.push(block.clone());
            }
            let most_recent_height = output_blocks[output_blocks.len() - 1].height();
            self.head_height = most_recent_height.increment();
            self.remove_data_below_height(self.head_height);
            return Some(output_blocks);
        }
        None
    }

    /// Check to see if the block is already present in the queue
    fn is_block_present(&mut self, block: &SequencerBlockSubset) -> bool {
        let block_hash = block.block_hash();
        let height = block.height();

        // check if the block is already present in the pending blocks
        if let Some(pending_blocks) = self.pending_blocks.get(&height) {
            if let Some(_block) = pending_blocks.get(&block_hash) {
                return true;
            }
        }
        // check if the block is already present in the soft blocks
        if let Some(soft_block) = self.soft_blocks.get(&height) {
            if soft_block.block_hash() == block_hash {
                return true;
            }
        }

        false
    }

    /// Check if there is another block in the pending blocks at a lower height
    /// that points to this block as its parent.
    fn is_block_a_parent(&mut self, block: SequencerBlockSubset) -> bool {
        let block_hash = block.block_hash();
        if let Some(child_blocks) = self.pending_blocks.get(&block.child_height()) {
            let blocks = child_blocks.values();
            for block in blocks {
                if let Some(hash) = block.parent_hash() {
                    if hash == block_hash {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Safely insert a block into the pending blocks. This function will insert
    /// the block into existing maps or create a new one if needed.
    fn insert_to_pending_blocks(&mut self, block: SequencerBlockSubset) {
        let height = block.height();
        let block_hash = block.block_hash();

        if let Some(pending_blocks) = self.pending_blocks.get_mut(&height) {
            let mut new_map = pending_blocks.clone();
            new_map.insert(block_hash, block);
            self.pending_blocks.insert(height, new_map);
        } else {
            let mut new_map = HashMap::new();
            new_map.insert(block_hash, block);
            self.pending_blocks.insert(height, new_map);
        }
        self.update_internal_state();
    }

    // remove all data in the queue below a given height. this does not remove
    // data from the soft queue, only the pending queue, and updates the head height.
    // TODO: add error handling
    fn remove_data_below_height(&mut self, height: Height) {
        // remove all data below the new incoming block from the pending data
        let tmp_pending = self.pending_blocks.clone();
        let mut pending_keys: Vec<&Height> = tmp_pending.keys().collect();
        pending_keys.sort();
        for key in pending_keys {
            if *key < height {
                self.pending_blocks.remove(key);
            }
        }
    }

    /// This function organizes the internal state of the queue based on the
    /// tendermint/CometBTF fork choice rules.
    ///
    /// Once a block is added to the pending_blocks in the queue, this function
    /// is called. It walks the pending blocks from lowest to highest height,
    /// checking to see if there is a continuous chain of blocks. For every block
    /// that is a descendant of the most recent "soft" block, and has a direct
    /// descendant, that block gets added to the `soft_blocks` BTreeMap and the
    /// head height is updated.
    fn update_internal_state(&mut self) {
        // check if the block added connects blocks in the pending queue
        let mut heights: Vec<Height> = self.pending_blocks.keys().cloned().collect();
        heights.sort();
        // walk the pending blocks starting from the head height
        for height in heights {
            // if the very first height in the pending blocks is the head height
            if height == self.head_height {
                if let Some(pending_blocks) = self.pending_blocks.clone().get(&height) {
                    // walk the pending blocks at that height and check if any of them are a parent
                    for block in pending_blocks.values() {
                        if self.is_block_a_parent(block.clone()) {
                            self.soft_blocks.insert(height, block.clone());
                            self.most_recent_soft_hash = block.block_hash();
                            self.head_height = height.increment();
                            self.remove_data_below_height(self.head_height);
                            break;
                        }
                    }
                }
            // if the first height in the queue is not the head height just stop reorg
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::HashSet,
        sync::Arc,
    };

    use astria_proto::generated::execution::v1alpha1::{
        DoBlockResponse,
        InitStateResponse,
    };
    use astria_sequencer_types::Namespace;
    use color_eyre::eyre::Result;
    use prost_types::Timestamp;
    use sha2::Digest as _;
    use tendermint::{
        block::Id as BlockId,
        Time,
    };
    use tokio::sync::{
        mpsc,
        Mutex,
    };

    use super::*;
    use crate::executor::{
        ExecutionClient,
        Executor,
    };

    // a mock ExecutionClient used for testing the Executor
    struct MockExecutionClient {
        finalized_blocks: Arc<Mutex<HashSet<Vec<u8>>>>,
    }

    impl MockExecutionClient {
        fn new() -> Self {
            Self {
                finalized_blocks: Arc::new(Mutex::new(HashSet::new())),
            }
        }
    }

    impl crate::private::Sealed for MockExecutionClient {}

    #[async_trait::async_trait]
    impl ExecutionClient for MockExecutionClient {
        // returns the sha256 hash of the prev_block_hash
        // the Executor passes self.execution_state as prev_block_hash
        async fn call_do_block(
            &mut self,
            prev_block_hash: Vec<u8>,
            _transactions: Vec<Vec<u8>>,
            _timestamp: Option<Timestamp>,
        ) -> Result<DoBlockResponse> {
            let res = hash(&prev_block_hash);
            Ok(DoBlockResponse {
                block_hash: res.to_vec(),
            })
        }

        async fn call_finalize_block(&mut self, block_hash: Vec<u8>) -> Result<()> {
            self.finalized_blocks.lock().await.insert(block_hash);
            Ok(())
        }

        async fn call_init_state(&mut self) -> Result<InitStateResponse> {
            let hasher = sha2::Sha256::new();
            Ok(InitStateResponse {
                block_hash: hasher.finalize().to_vec(),
            })
        }
    }

    /// Return the number of blocks in the queue
    fn queue_len(queue: Queue) -> usize {
        let pending_blocks = queue.pending_blocks;
        let soft_blocks = queue.soft_blocks;
        let mut len = 0;
        for height in pending_blocks.values() {
            len += height.keys().len();
        }
        len += soft_blocks.len();
        len
    }

    fn hash(s: &[u8]) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(s);
        hasher.finalize().to_vec()
    }

    fn get_test_block_subset() -> SequencerBlockSubset {
        SequencerBlockSubset {
            block_hash: hash(b"block1").try_into().unwrap(),
            header: astria_sequencer_types::test_utils::default_header(),
            rollup_transactions: vec![],
        }
    }

    // build a vec of sequential blocks for testing
    fn get_test_block_vec(num_blocks: u32) -> Vec<SequencerBlockSubset> {
        // let namespace = Namespace::from_slice(b"test");

        let mut block = get_test_block_subset();
        block.rollup_transactions.push(b"test_transaction".to_vec());

        let mut blocks = vec![];

        block.header.height = 1_u32.into();
        blocks.push(block);

        for i in 2..=num_blocks {
            let current_hash_string = String::from("block") + &i.to_string();
            let prev_hash_string = String::from("block") + &(i - 1).to_string();
            let current_byte_hash: &[u8] = &current_hash_string.into_bytes();
            let prev_byte_hash: &[u8] = &prev_hash_string.into_bytes();

            let mut block = get_test_block_subset();
            block.block_hash = Hash::try_from(hash(current_byte_hash)).unwrap();
            block.rollup_transactions.push(b"test_transaction".to_vec());

            block.header.height = i.into();
            let block_id = BlockId {
                hash: Hash::try_from(hash(prev_byte_hash)).unwrap(),
                ..Default::default()
            };
            block.header.last_block_id = Some(block_id);

            blocks.push(block);
        }
        blocks
    }

    // test that executing consecutive blocks works and also doesn't leave any
    // of those blocks in the queue
    #[tokio::test]
    async fn insert_next_block() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::from_slice(b"test");
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
            .await
            .unwrap();

        let blocks = get_test_block_vec(2);

        // because the block is executed the execution state is updated
        let mut expected_execution_hash = hash(&executor.execution_state);
        let execution_block_hash = executor
            // insert the first block
            .execute_block(blocks[0].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, execution_block_hash);
        // because the block can be executed it does not stay in the queue
        assert_eq!(queue_len(executor.block_queue.clone()), 0);

        expected_execution_hash = hash(&executor.execution_state);
        let execution_block_hash_1 = executor
            // insert the first block
            .execute_block(blocks[1].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, execution_block_hash_1);
        // because the block can be executed it does not stay in the queue
        assert_eq!(queue_len(executor.block_queue.clone()), 0);
    }

    // trying to execute a non-consecutive block doesn't change the execution
    // state and that block is added to the queue, extending its length
    #[tokio::test]
    async fn insert_not_next_block() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::from_slice(b"test");
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
            .await
            .unwrap();

        let blocks = get_test_block_vec(2);

        // because the block is out of order it is added to the queue and the
        // execution state doesn't change
        let expected_execution_hash = executor.execution_state.clone();
        let execution_block_hash = executor
            // inserting block 2 when we haven't seen block 1
            .execute_block(blocks[1].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, execution_block_hash);
        // because the block is out of order it is added to the queue
        assert_eq!(queue_len(executor.block_queue.clone()), 1);

        // the out of order block is not executed so it's hash is not in the
        // sequencer hash to execution hash map
        let sequencer_block_hash = blocks[1].block_hash();
        let missing_block_execution_hash = executor
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash);
        assert_eq!(missing_block_execution_hash, None);
    }

    // test that filling a gap in the queued blocks, executes all consecutive
    // blocks and clears the queue
    #[tokio::test]
    async fn fill_block_gap() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::from_slice(b"test");
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
            .await
            .unwrap();

        let blocks = get_test_block_vec(2);

        // add an out of order block to the queue
        let expected_execution_hash = executor.execution_state.clone();
        let execution_block_hash_1 = executor
            .execute_block(blocks[1].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, execution_block_hash_1);
        assert_eq!(queue_len(executor.block_queue.clone()), 1);

        // executing the skipped block
        // the execution state is updated twice because multiple blocks are
        // executed. see hash(hash()) on next line
        let expected_execution_hash = hash(&hash(&executor.execution_state));
        let expected_execution_hash_of_missing_block = hash(&executor.execution_state);
        let execution_block_hash_0 = executor
            .execute_block(blocks[0].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, execution_block_hash_0);
        // check that the execution hash of the missing block is still in the
        // sequencer_hash_to_execution_hash map
        let sequencer_block_hash = blocks[0].block_hash();
        let missing_block_execution_hash = executor
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .unwrap()
            .clone();
        assert_eq!(
            missing_block_execution_hash,
            expected_execution_hash_of_missing_block
        );
    }

    #[tokio::test]
    async fn fill_multiple_block_gaps_in_reverse_order() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::from_slice(b"test");
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
            .await
            .unwrap();

        let blocks = get_test_block_vec(5);

        let original_execution_state = executor.execution_state.clone();

        // add an out of order block to the queue
        let expected_execution_hash = executor.execution_state.clone();
        let execution_block_hash_1 = executor
            .execute_block(blocks[1].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, execution_block_hash_1);
        assert_eq!(queue_len(executor.block_queue.clone()), 1);

        // add another out of order block to the queue with another gap
        let expected_execution_hash = executor.execution_state.clone();
        let execution_block_hash_3 = executor
            .execute_block(blocks[3].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, execution_block_hash_3);
        assert_eq!(queue_len(executor.block_queue.clone()), 2);

        // add a block that fills the second gaps but not the
        // first. execution state shouldn't change yet
        let expected_execution_hash = executor.execution_state.clone();
        let execution_block_hash_2 = executor
            .execute_block(blocks[2].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, execution_block_hash_2);
        assert_eq!(queue_len(executor.block_queue.clone()), 3);

        // add the final missing block to the queue
        // all the block in the queue should be executed and the queue should be cleared
        let expected_execution_hash = hash(&hash(&hash(&hash(&executor.execution_state))));
        let execution_block_hash_0 = executor
            .execute_block(blocks[0].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        // the returned execution hash is the hash of the execution state after
        // the most recent block is executed (block 4)
        assert_eq!(expected_execution_hash, execution_block_hash_0);
        assert_eq!(queue_len(executor.block_queue.clone()), 0);

        // check that the execution hash of all the blocks are in the
        // sequencer_hash_to_execution_hash map
        // block 1 has an exeuction hash
        let sequencer_block_hash = blocks[0].block_hash();
        let expected_state = hash(&original_execution_state);
        let block_execution_hash = executor
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .unwrap()
            .clone();
        assert_eq!(expected_state, block_execution_hash);
        // block 2 has an exeuction hash
        let sequencer_block_hash = blocks[1].block_hash();
        let expected_state = hash(&hash(&original_execution_state));
        let block_execution_hash = executor
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .unwrap()
            .clone();
        assert_eq!(expected_state, block_execution_hash);
        // block 3 has an exeuction hash
        let sequencer_block_hash = blocks[2].block_hash();
        let expected_state = hash(&hash(&hash(&original_execution_state)));
        let block_execution_hash = executor
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .unwrap()
            .clone();
        assert_eq!(expected_state, block_execution_hash);
        // block 4 has an exeuction hash
        let sequencer_block_hash = blocks[3].block_hash();
        let expected_state = hash(&hash(&hash(&hash(&original_execution_state))));
        let block_execution_hash = executor
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .unwrap()
            .clone();
        assert_eq!(expected_state, block_execution_hash);
    }

    #[tokio::test]
    async fn fork_choice_head_setting() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::from_slice(b"test");
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
            .await
            .unwrap();

        let blocks = get_test_block_vec(4);

        // because the block is executed the execution state is updated
        let mut expected_execution_hash = hash(&executor.execution_state);
        let execution_block_hash = executor
            // insert the first block
            .execute_block(blocks[0].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, execution_block_hash);
        // because the block can be executed it does not stay in the queue
        assert_eq!(queue_len(executor.block_queue.clone()), 0);

        // add a block that doesn't have a parent
        let execution_block_hash_2a = executor
            .execute_block(blocks[2].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        // execution hash not updated
        assert_eq!(expected_execution_hash, execution_block_hash_2a);
        assert_eq!(queue_len(executor.block_queue.clone()), 1);

        // add in the same block again with a newer timestamp
        // this simulates a different block at the same height
        let mut newer_2_block = blocks[2].clone();
        newer_2_block.header.time = Time::now();
        newer_2_block.block_hash = Hash::try_from(hash(b"some_other_hash")).unwrap();
        let execution_block_hash_2b = executor
            .execute_block(newer_2_block)
            .await
            .unwrap()
            .expect("expected execution block hash");
        // execution hash not updated
        assert_eq!(expected_execution_hash, execution_block_hash_2b);
        assert_eq!(queue_len(executor.block_queue.clone()), 2);

        // now when the missing block arrives, all the blocks get executed
        // because everything at the head height is sent to execution
        expected_execution_hash = hash(&hash(&hash(&executor.execution_state)));
        let execution_block_hash_1 = executor
            .execute_block(blocks[1].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, execution_block_hash_1);
        // and the queue gets executed and cleared. the second block at height 2
        // is cleared
        assert_eq!(queue_len(executor.block_queue.clone()), 0);

        // execute another block after the head with multiple blocks
        expected_execution_hash = hash(&executor.execution_state);
        let execution_block_hash = executor
            // insert the first block
            .execute_block(blocks[3].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, execution_block_hash);
        // because the block can be executed it does not stay in the queue
        assert_eq!(queue_len(executor.block_queue.clone()), 0);
    }

    // TODO (GHI #207: https://github.com/astriaorg/astria/issues/207)
    // add a new test to check that the `execution_commit_level` setting
    // actually changes the execution behavior
    // -> fn fork_choice_soft_setting() {blah}
}
