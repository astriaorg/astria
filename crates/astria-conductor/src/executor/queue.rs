use std::collections::{
    BTreeMap,
    HashMap,
};

use astria_sequencer_relayer::types::SequencerBlockData;
use tendermint::{
    block::Height,
    hash::Hash,
};
use tracing::{
    debug,
    info,
};

enum ExecutorQueueParentStatus {
    ParentPending(Box<SequencerBlockData>),
    ParentSoft,
    NoParent,
}

// TODO: update this description
/// A queue for the SequencerBlockData type that is holds blocks that are not
/// yett ready for execution.
///
/// The queue is implemented
pub struct ExecutorQueue {
    head_height: Height,
    most_recent_soft_hash: Hash, // ??? not sure if we still need this

    // the collection of all pending blocks. the lowest height in this map is
    // the head blocks
    pending_blocks: HashMap<Height, HashMap<Hash, SequencerBlockData>>,
    // all blocks in order by height that can be considered safe because they
    // have a parent
    soft_blocks: BTreeMap<Height, SequencerBlockData>,
}

impl ExecutorQueue {
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
    /// Returns None if the block was added to the queue.
    /// Returns Some(Hash) if the block was already present in the queue. The
    /// Hash will be the hash of the block that was already present in the queue.
    // TODO: add error handling
    pub(super) fn insert(&mut self, block: SequencerBlockData) {
        // if the block is already in the queue, return its hash
        if self.is_block_present(block.clone()) {
            // return Some(get_block_hash(block.clone()));
            info!(
                "block with height {} and hash {} is already present in the queue",
                get_block_height(block.clone()),
                get_block_hash(block.clone())
            );
        }

        if get_block_height(block.clone()) < self.head_height() {
            info!(
                "block with height {} is stale and will not be added to the queue",
                get_block_height(block)
            );
        }

        match self.check_if_parent_present(block.clone()) {
            ExecutorQueueParentStatus::ParentPending(parent_block) => {
                self.insert_and_update_pending_blocks(block, *parent_block);
            }
            ExecutorQueueParentStatus::ParentSoft => {
                self.insert_and_update_soft_queue(block);
            }
            // if the block has no parent, add it to the pending blocks without
            // updating other data
            ExecutorQueueParentStatus::NoParent => {
                self.insert_to_pending_blocks(block);
            }
        }
        info!(
            "block with height {} and hash {} was added to the queue",
            get_block_height(block.clone()),
            get_block_hash(block.clone())
        );
    }

    // check to see if the block is already present in the queue
    fn is_block_present(&mut self, block: &SequencerBlockData) -> bool {
        let block_hash = get_block_hash(block.clone());
        let height = get_block_height(block);

        // check if the block is already present in the pending blocks
        if let Some(pending_blocks) = self.pending_blocks.get(&height) {
            if let Some(_block) = pending_blocks.get(&block_hash) {
                return true;
            }
        }
        // check if the block is already present in the soft blocks
        if let Some(soft_block) = self.soft_blocks.get(&height) {
            if get_block_hash(soft_block.clone()) == block_hash {
                return true;
            }
        }

        false
    }

    // check if the parent of the incoming block is present in the queue and
    // return that parent block if it is
    fn check_if_parent_present(&mut self, block: SequencerBlockData) -> ExecutorQueueParentStatus {
        let parent_height = get_parent_block_height(block.clone());
        if let Some(parent_hash) = get_parent_block_hash(block) {
            // check if parent in pending data
            if let Some(pending_blocks) = self.pending_blocks.get(&parent_height) {
                if let Some(parent_block) = pending_blocks.get(&parent_hash) {
                    return ExecutorQueueParentStatus::ParentPending(Box::new(
                        parent_block.clone(),
                    ));
                }
            }
            // check if parent in soft data
            if let Some(soft_block) = self.soft_blocks.get(&parent_height) {
                if get_block_hash(soft_block.clone()) == parent_hash {
                    return ExecutorQueueParentStatus::ParentSoft;
                }
            }
        }

        ExecutorQueueParentStatus::NoParent
    }

    fn is_block_a_parent(&mut self, block: SequencerBlockData) -> bool {
        let block_hash = get_block_hash(block.clone());
        if let Some(child_blocks) = self.pending_blocks.get(&get_child_block_height(block)) {
            let blocks = child_blocks.values();
            for block in blocks {
                if let Some(hash) = get_parent_block_hash(block.clone()) {
                    if hash == block_hash {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn update_head_height(&mut self, height: Height) {
        self.head_height = height;
    }

    fn update_most_recent_soft_hash(&mut self, hash: Hash) {
        self.most_recent_soft_hash = hash;
    }

    fn head_height(&mut self) -> Height {
        self.head_height
    }

    // fn head_height_plus_one(&mut self) -> Height {
    //     Height::try_from(self.head_height.value() + 1).expect("could not convert u64 to Height")
    // }

    // a basic insert into the pending blocks
    // TODO: update for error handling
    fn insert_to_pending_blocks(&mut self, block: SequencerBlockData) {
        let height = get_block_height(block.clone());
        let block_hash = get_block_hash(block.clone());

        if let Some(pending_blocks) = self.pending_blocks.get_mut(&height) {
            let mut new_map = pending_blocks.clone();
            new_map.insert(block_hash, block);
            self.pending_blocks.insert(height, new_map);
        } else {
            let mut new_map = HashMap::new();
            new_map.insert(block_hash, block);
            self.pending_blocks.insert(height, new_map);
        }

        // TODO: add a call to update the queue
    }

    // TODO: update description
    fn insert_and_update_pending_blocks(
        &mut self,
        block: SequencerBlockData,
        parent_block: SequencerBlockData,
    ) -> Hash {
        let height = get_block_height(block.clone());
        let block_hash = get_block_hash(block.clone());
        let parent_height = get_block_height(parent_block.clone());
        let parent_hash = get_block_hash(parent_block.clone());

        // remove parent data from pending blocks
        if let Some(pending_blocks) = self.pending_blocks.get_mut(&parent_height) {
            pending_blocks.remove(&parent_hash);
        }
        // add parent block to the soft queue
        self.soft_blocks.insert(parent_height, parent_block);
        self.update_most_recent_soft_hash(parent_hash);

        // remove all other data below the new incoming block
        self.remove_data_blow_height(height);
        // remove all blocks in the head queue that don't have the most recent soft block as their
        // parent
        self.clean_head_blocks();

        // check if the new block is a parent of any of the pending blocks
        if self.is_block_a_parent(block.clone()) {
            self.remove_data_blow_height(get_child_block_height(block.clone()));
            // TODO: update this to used the update and insert to soft blocks function
            self.soft_blocks.insert(height, block.clone());
            self.update_most_recent_soft_hash(block_hash);
            self.clean_head_blocks();
        } else {
            // add the new block to the pending blocks
            self.insert_to_pending_blocks(block);
        }

        block_hash
    }

    // remove all blocks in the head queue that don't have the most recent soft
    // block as their parent
    fn clean_head_blocks(&mut self) {
        if let Some(head_blocks) = self.pending_blocks.get_mut(&self.head_height) {
            let tmp_blocks = head_blocks.clone();
            let blocks = tmp_blocks.values();
            for block in blocks {
                if let Some(parent_hash) = get_parent_block_hash(block.clone()) {
                    if parent_hash != self.most_recent_soft_hash {
                        head_blocks.remove(&get_block_hash(block.clone()));
                    }
                }
            }
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

        self.update_head_height(height);
    }

    // TODO: fix this to actually be correct
    fn insert_and_update_soft_queue(&mut self, block: SequencerBlockData) {
        self.soft_blocks
            .insert(get_block_height(block.clone()), block.clone());

        if self.is_block_a_parent(block.clone()) {
            let block_hash = get_block_hash(block.clone());
            let block_height = get_block_height(block.clone());

            self.remove_data_blow_height(get_child_block_height(block.clone()));
            // TODO: update this to used the update and insert to soft blocks function
            self.soft_blocks.insert(block_height, block.clone());
            self.update_most_recent_soft_hash(block_hash);
            self.clean_head_blocks();
        } else {
            // add the new block to the pending blocks
            self.insert_to_pending_blocks(block);
        }
    }

    /// Return all valid blocks ("soft blocks") and "Head" blocks that are ready
    /// to be executed.
    ///
    /// WARNING: This function removes the blocks that it returns from the
    /// queue.
    /// This function returns an `Option<Vec<SequencerBlockData>>`. A `Some`
    /// value contains a vector of `SequencerBlockData` that are ready to be
    /// executed. A `None` value indicates that there are no blocks in the queue.
    pub(super) fn get_blocks(&mut self) -> Option<Vec<SequencerBlockData>> {
        // return everything before the head height AND all data at H+1
        let mut output_blocks: Vec<SequencerBlockData> = vec![];
        if let Some(head_blocks) = self.pending_blocks.get(&self.head_height) {
            let tmp_blocks = head_blocks.clone();
            let mut blocks: Vec<SequencerBlockData> = tmp_blocks.values().cloned().collect();
            output_blocks.append(blocks.as_mut());
            self.pending_blocks.remove(&self.head_height);
            // don't need to update any data about the head yet because none of
            // those blocks are technically "safe" yet.
            // TODO: this will send a lot of the same blocks multiple times if
            // there are a lot of blocks in the head queue. Need a way to track
            // which head blocks have already been sent.
        }
        if let Some(mut soft_blocks) = self.get_soft_blocks() {
            output_blocks.append(soft_blocks.as_mut());
            self.soft_blocks.clear();
        }
        output_blocks.sort();
        if !output_blocks.is_empty() {
            Some(output_blocks)
        } else {
            None
        }
    }

    // TODO: this will return all the blocks in the soft queue that are already "safe"
    pub(super) fn get_soft_blocks(&mut self) -> Option<Vec<SequencerBlockData>> {
        let mut soft_blocks: Vec<SequencerBlockData> = self.soft_blocks.values().cloned().collect();
        soft_blocks.sort();
        if !soft_blocks.is_empty() {
            Some(soft_blocks)
        } else {
            None
        }
    }

    // TODO: it is worth add a "peek" that returns blocks but doesn't delete them?

    /// Return the number of blocks in the queue
    pub(super) fn len(&self) -> usize {
        let mut len = 0;
        len += self.pending_blocks.len();
        len += self.soft_blocks.len();
        len
    }

    pub(super) fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// block data helper functions
// TODO: get rid of this in the code and use the block data directly
fn get_block_height(block: &SequencerBlockData) -> Height {
    block.header.height
}

fn get_block_hash(block: SequencerBlockData) -> Hash {
    Hash::try_from(block.block_hash).expect("block must have a block hash")
}

fn get_parent_block_height(block: SequencerBlockData) -> Height {
    Height::try_from(block.header.height.value() - 1).expect("could not convert u64 to Height")
}

fn get_child_block_height(block: SequencerBlockData) -> Height {
    Height::try_from(block.header.height.value() + 1).expect("could not convert u64 to Height")
}

fn get_parent_block_hash(block: SequencerBlockData) -> Option<Hash> {
    if let Some(parent_hash) = block.header.last_block_id {
        return Some(parent_hash.hash);
    } else {
        // all incoming blocks should have a parent, it is ignored if it doesn't
        if let Ok(hash) = std::str::from_utf8(block.block_hash.as_slice()) {
            debug!(hash = hash.to_string(), "block has no parent");
        } else {
            debug!("block hash cannot be parsed");
        }
    }
    None
}

// TODO: add a test for the tree and the queue
