use std::collections::BTreeMap;

use tendermint::{
    hash::Hash,
};
use tracing::{
    debug,
    info,
};

use astria_sequencer_types::SequencerBlockData;


#[derive(Debug, Clone)]
pub(super) struct Queue {
    // Internal var tracks the next executable block on the chain
    exec_height: u64,
    // All blocks in order by height that can be considered safe because they
    // have a child block
    blocks: BTreeMap<u64, SequencerBlockData>,
}

impl Queue {
    pub(super) fn new(exec_height: u64) -> Self {
        Self {
            exec_height,
            blocks: BTreeMap::new(),
        }
    }


    pub(super) fn insert(&mut self, block: SequencerBlockData) -> Option<Hash> {
        // if the block is already in the queue, return its hash
        if self.is_block_present(&block) {
            debug!(
                block.height = %block.header().height,
                block.hash = %block.block_hash(),
                "block is already present in the queue"
            );
            return None;
        }

        // if the block is stale, ignore it
        if block.header().height.value() < self.exec_height {
            debug!(
                block.height = %block.header().height,
                "block is stale and will not be added to the queue"
            );
            return None;
        }

        self.blocks.insert(block.header().height.value(), block.clone());
        info!(
            block.height = %block.header().height,
            block.hash = %block.block_hash(),
            "block added to queue"
        );
        Some(block.block_hash())
    }

    pub(super) fn increment_head_height(&mut self) {
        self.exec_height += 1;
    }

    pub(super) fn get_missing_block_end(&self) -> Option<(u64, u64)> {
        if let Some((&top_height, _)) = self.blocks.first_key_value() {
            if top_height == self.exec_height {
                return None;
            }

            return Some((self.exec_height, top_height));
        }

        Some((self.exec_height, self.exec_height))
    }

    pub(super) fn get_executable_block(&mut self) -> Option<SequencerBlockData> {
        if let Some((&top_height, _)) = self.blocks.first_key_value() {
            if top_height == self.exec_height {
                if let Some((_ , block)) = self.blocks.pop_first() {
                    return Some(block);
                }
            }
        }
        None
    }

    /// Check to see if the block is already present in the queue
    fn is_block_present(&self, block: &SequencerBlockData) -> bool {
        let block_hash = block.block_hash();
        let height = block.header().height.value();

        // check if the block is already present in the soft blocks
        if let Some(block) = self.blocks.get(&height) {
            if block.block_hash() == block_hash {
                return true;
            }
        }

        false
    }
}
