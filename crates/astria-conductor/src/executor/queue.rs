use std::collections::{
    BTreeMap,
    HashMap,
};

use tendermint::{
    block::Height,
    hash::Hash,
};
use tracing::info;

use crate::types::SequencerBlockSubset;

/// A queue for the SequencerBlockSubset type that holds blocks that are
/// pending or not yet ready for execution.
///
/// This Queue handles all the fork choice logic for incoming Sequencer blocks
/// from gossip. It is responsible for determining which blocks are safe to
/// pass on to execution, and holds on to the other data until it is safe to
/// execute, or deletes it if it is no longer needed or becomes stale or invalid.
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
    /// and arrange all blocks in the queue, base on the tendermin/CometBFT fork
    /// choice rules.
    pub(super) fn insert(&mut self, block: SequencerBlockSubset) {
        // if the block is already in the queue, return its hash
        if self.is_block_present(&block) {
            info!(
                block.height = %block.height(),
                block.hash = %block.block_hash(),
                "block is already present in the queue"
            );
        }

        // if the block is stale, ignore it
        if block.header().height < self.head_height {
            info!(
                block.height = %block.height(),
                "block is stale and will not be added to the queue"
            );
        }

        // if the block is at the head height OR in the future, just add it to
        // the pending blocks
        self.insert_to_pending_blocks(block.clone());

        self.update_internal_state();

        info!(
            block.height = %block.height(),
            block.hash = %block.block_hash(),
            "block added to queue"
        );
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

    // TODO: this will return all the blocks in the soft queue that are already "safe"
    pub(super) fn pop_soft_blocks(&mut self) -> Option<Vec<SequencerBlockSubset>> {
        let mut soft_blocks: Vec<SequencerBlockSubset> =
            self.soft_blocks.values().cloned().collect();
        if !soft_blocks.is_empty() {
            soft_blocks.sort();
            self.soft_blocks.clear();
            let highest_soft_block = soft_blocks[soft_blocks.len() - 1].clone();
            self.head_height = highest_soft_block.height().increment();
            self.remove_data_blow_height(self.head_height);
            Some(soft_blocks)
        } else {
            None
        }
    }

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
            self.remove_data_blow_height(self.head_height);
            return Some(output_blocks);
        }
        None
    }

    // check to see if the block is already present in the queue
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
    }

    // remove all data in the queue below a given height. this does not remove
    // data from the soft queue, only the pending queue, and updates the head height.
    // TODO: add error handling
    fn remove_data_blow_height(&mut self, height: Height) {
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
    /// is called. It walks the panding blocks from lowest to highest height,
    /// checking to see if there is a continues chain of blocks. For every block
    /// that is a decendant of the most recent "soft" block, and has a direct
    /// decendant, that block gets added to the `soft_blocks` BTreeMap and the
    /// head height is updated.
    /// after, and including, the most recent "soft" block that has a direct child
    fn update_internal_state(&mut self) {
        // check if the block added connects blocks in the pending queue
        let mut heights: Vec<Height> = self.pending_blocks.keys().cloned().collect();
        heights.sort();
        for height in heights {
            // if the very first height in the pending blocks is the head
            if height == self.head_height {
                if let Some(pending_blocks) = self.pending_blocks.clone().get(&height) {
                    for block in pending_blocks.values() {
                        if self.is_block_a_parent(block.clone()) {
                            self.soft_blocks.insert(height, block.clone());
                            self.most_recent_soft_hash = block.block_hash();
                            self.head_height = height.increment();
                            self.remove_data_blow_height(self.head_height);
                            break;
                        }
                    }
                }
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

    #[tokio::test]
    async fn insert_next_block() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::from_slice(b"test");
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
            .await
            .unwrap();

        let blocks = get_test_block_vec(2);

        // because the block is executed the execution state is updated
        let mut expected_exection_hash = hash(&executor.execution_state);
        let execution_block_hash = executor
            // insert the first block
            .execute_block(blocks[0].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash);
        // because the block can be executed it does not stay in the queue
        assert_eq!(queue_len(executor.block_queue.clone()), 0);

        expected_exection_hash = hash(&executor.execution_state);
        let execution_block_hash_1 = executor
            // insert the first block
            .execute_block(blocks[1].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_1);
        // because the block can be executed it does not stay in the queue
        assert_eq!(queue_len(executor.block_queue.clone()), 0);
    }

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
        let expected_exection_hash = executor.execution_state.clone();
        let execution_block_hash = executor
            // inserting block 2 when we haven't seen block 1
            .execute_block(blocks[1].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash);
        // because the block is out of order it is added to the queue
        assert_eq!(queue_len(executor.block_queue.clone()), 1);
    }

    #[tokio::test]
    async fn fill_block_gap() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::from_slice(b"test");
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
            .await
            .unwrap();

        let blocks = get_test_block_vec(2);

        // add an out of order block to the queue
        let expected_exection_hash = executor.execution_state.clone();
        let execution_block_hash_1 = executor
            .execute_block(blocks[1].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_1);
        assert_eq!(queue_len(executor.block_queue.clone()), 1);

        // executing a block like normal
        let expected_exection_hash = hash(&hash(&executor.execution_state));
        let expected_exection_hash_of_missing_block = hash(&executor.execution_state);
        let execution_block_hash_0 = executor
            .execute_block(blocks[0].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_0);
        let sequencer_block_hash = blocks[0].block_hash();
        let missing_block_execution_hash = executor
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .unwrap()
            .clone();
        assert_eq!(
            missing_block_execution_hash,
            expected_exection_hash_of_missing_block
        );
    }

    #[tokio::test]
    async fn fill_multiple_block_gaps() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::from_slice(b"test");
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
            .await
            .unwrap();

        let blocks = get_test_block_vec(5);

        // add an out of order block to the queue
        let expected_exection_hash = executor.execution_state.clone();
        let execution_block_hash_1 = executor
            .execute_block(blocks[1].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_1);
        assert_eq!(queue_len(executor.block_queue.clone()), 1);

        // add another out of order block to the queue with another gap
        let expected_exection_hash = executor.execution_state.clone();
        let execution_block_hash_3 = executor
            .execute_block(blocks[3].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_3);
        assert_eq!(queue_len(executor.block_queue.clone()), 2);

        // executing a block like normal
        let expected_exection_hash = hash(&hash(&executor.execution_state));
        let expected_exection_hash_of_missing_block = hash(&executor.execution_state);
        let execution_block_hash_0 = executor
            .execute_block(blocks[0].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_0);
        let sequencer_block_hash = blocks[0].block_hash();
        let missing_block_execution_hash = executor
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .unwrap()
            .clone();
        assert_eq!(
            missing_block_execution_hash,
            expected_exection_hash_of_missing_block
        );

        // executing a block like normal
        let expected_exection_hash = hash(&hash(&executor.execution_state));
        let expected_exection_hash_of_missing_block = hash(&executor.execution_state);
        let execution_block_hash_2 = executor
            .execute_block(blocks[2].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_2);
        let sequencer_block_hash = blocks[2].block_hash();
        let missing_block_execution_hash = executor
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .unwrap()
            .clone();
        assert_eq!(
            missing_block_execution_hash,
            expected_exection_hash_of_missing_block
        );
    }

    #[tokio::test]
    async fn fork_chioce_head_setting() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::from_slice(b"test");
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
            .await
            .unwrap();

        let blocks = get_test_block_vec(4);

        // because the block is executed the execution state is updated
        let mut expected_exection_hash = hash(&executor.execution_state);
        let execution_block_hash = executor
            // insert the first block
            .execute_block(blocks[0].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash);
        // because the block can be executed it does not stay in the queue
        assert_eq!(queue_len(executor.block_queue.clone()), 0);

        // add a block that doesn't have a parent
        let execution_block_hash_2a = executor
            .execute_block(blocks[2].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        // exectuion hash not updated
        assert_eq!(expected_exection_hash, execution_block_hash_2a);
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
        // exectuion hash not updated
        assert_eq!(expected_exection_hash, execution_block_hash_2b);
        assert_eq!(queue_len(executor.block_queue.clone()), 2);

        // now when the missing block arrives, all the blocks get executed
        // because everything at the head height is sent to execution
        expected_exection_hash = hash(&hash(&hash(&executor.execution_state)));
        let execution_block_hash_1 = executor
            .execute_block(blocks[1].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_1);
        // and the queue gets executed and cleared. the second block at height 2
        // is cleared
        assert_eq!(queue_len(executor.block_queue.clone()), 0);

        // execute another block after the head with multiple blocks
        expected_exection_hash = hash(&executor.execution_state);
        let execution_block_hash = executor
            // insert the first block
            .execute_block(blocks[3].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash);
        // because the block can be executed it does not stay in the queue
        assert_eq!(queue_len(executor.block_queue.clone()), 0);
    }

    // TODO (GHI #207: https://github.com/astriaorg/astria/issues/207)
    // add a new test to check that the `execution_commit_level` setting
    // actually changes the execution behavior
    // -> fn fork_choice_soft_setting() {blah}

    // #[tokio::test]
    // async fn test_block_queue() {
    // let (alert_tx, _) = mpsc::unbounded_channel();
    // let namespace = Namespace::from_slice(b"test");
    // let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
    //     .await
    //     .unwrap();

    // let blocks = get_test_block_vec(10);

    // // executing a block like normal
    // let mut expected_exection_hash = hash(&executor.execution_state);
    // let execution_block_hash_0 = executor
    //     .execute_block(blocks[0].clone())
    //     .await
    //     .unwrap()
    //     .expect("expected execution block hash");
    // assert_eq!(expected_exection_hash, execution_block_hash_0);

    // // adding a block without a parent in the execution chain doesn't change the execution
    // // state and adds it to the queue
    // let execution_block_hash_2 = executor
    //     .execute_block(blocks[2].clone())
    //     .await
    //     .unwrap()
    //     .expect("expected execution block hash");
    // assert_eq!(expected_exection_hash, execution_block_hash_2);
    // assert_eq!(queue_len(executor.block_queue.clone()), 1);

    // // adding another block without a parent in the current chain (but does have parent in
    // the // queue). also doesn't change execution state
    // let execution_block_hash_3 = executor
    //     .execute_block(blocks[3].clone())
    //     .await
    //     .unwrap()
    //     .expect("expected execution block hash");
    // assert_eq!(expected_exection_hash, execution_block_hash_3);
    // assert_eq!(queue_len(executor.block_queue.clone()), 2);

    // // adding the actual next block updates the execution state
    // // using hash() 3 times here because adding the new block and executing the queue updates
    // // the state 3 times. one for each block.
    // expected_exection_hash = hash(&hash(&hash(&executor.execution_state)));
    // let execution_block_hash_1 = executor
    //     .execute_block(blocks[1].clone())
    //     .await
    //     .unwrap()
    //     .expect("expected execution block hash");
    // assert_eq!(expected_exection_hash, execution_block_hash_1);
    // // and the queue gets executed and cleared
    // assert_eq!(queue_len(executor.block_queue.clone()), 0);

    //     // a new block with a parent appears and is executed
    //     expected_exection_hash = hash(&executor.execution_state);
    //     let execution_block_hash_4 = executor
    //         .execute_block(blocks[4].clone())
    //         .await
    //         .unwrap()
    //         .expect("expected execution block hash");
    //     assert_eq!(expected_exection_hash, execution_block_hash_4);

    //     // add another block that doesn't have a parent
    //     let execution_block_hash_6 = executor
    //         .execute_block(blocks[6].clone())
    //         .await
    //         .unwrap()
    //         .expect("expected execution block hash");
    //     // exectuion hash not updated
    //     assert_eq!(expected_exection_hash, execution_block_hash_6);
    //     assert_eq!(queue_len(executor.block_queue.clone()), 1);

    //     // add in the same block again with a newer timestamp
    //     // this simulates a different block at the same height
    //     let mut newer_6_block = blocks[6].clone();
    //     newer_6_block.header.time = Time::now();
    //     let execution_block_hash_6 = executor
    //         .execute_block(newer_6_block)
    //         .await
    //         .unwrap()
    //         .expect("expected execution block hash");
    //     // exectuion hash not updated
    //     assert_eq!(expected_exection_hash, execution_block_hash_6);
    //     // the newer block replaces the block of the same height in the queue so the queue
    // doesn't     // grow
    //     assert_eq!(queue_len(executor.block_queue.clone()), 1);

    //     // add another block that doesn't have a parent and also a gap between the last block
    // added     // to the queue that doesn't have a parent
    //     let execution_block_hash_8 = executor
    //         .execute_block(blocks[8].clone())
    //         .await
    //         .unwrap()
    //         .expect("expected execution block hash");
    //     // exectuion hash not updated
    //     assert_eq!(expected_exection_hash, execution_block_hash_8);
    //     assert_eq!(queue_len(executor.block_queue.clone()), 2);

    //     // add a block that fills the first gap
    //     // in this case there are two 6 blocks but one is newer
    //     // only the latest block gets executed here and the old 6 block just gets deleted
    //     expected_exection_hash = hash(&hash(&executor.execution_state)); // 2 blocks executed
    //     let execution_block_hash_5 = executor
    //         .execute_block(blocks[5].clone())
    //         .await
    //         .unwrap()
    //         .expect("expected execution block hash");
    //     assert_eq!(expected_exection_hash, execution_block_hash_5);
    //     // only one block in the queue gets executed because there is still a gap
    //     assert_eq!(queue_len(executor.block_queue.clone()), 1);

    //     // add a block that fills the second gap
    //     expected_exection_hash = hash(&hash(&executor.execution_state));
    //     let execution_block_hash_7 = executor
    //         .execute_block(blocks[7].clone())
    //         .await
    //         .unwrap()
    //         .expect("expected execution block hash");
    //     assert_eq!(expected_exection_hash, execution_block_hash_7);
    //     // the rest of the queue is executed because all gaps are filled
    //     assert_eq!(queue_len(executor.block_queue.clone()), 0);

    //     // one final block executed like normal
    //     expected_exection_hash = hash(&executor.execution_state);
    //     let execution_block_hash_9 = executor
    //         .execute_block(blocks[9].clone())
    //         .await
    //         .unwrap()
    //         .expect("expected execution block hash");
    //     assert_eq!(expected_exection_hash, execution_block_hash_9);
    //     assert_eq!(queue_len(executor.block_queue.clone()), 0);
    // }
}
