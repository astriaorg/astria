use std::{
    collections::HashMap,
    mem,
};

use base64::{
    display::Base64Display,
    engine::general_purpose::STANDARD,
};
use tokio::time::{
    Duration,
    Instant,
};
use tracing::warn;

use crate::types::SequencerBlockData;

// the max time to wait for a block to finalize
const MAX_QUEUE_TIME: Duration = Duration::from_secs(10);

#[derive(Default)]
pub(crate) struct QueuedBlocks {
    pending_finalization: HashMap<Vec<u8>, (SequencerBlockData, Instant)>,
    finalized: Vec<SequencerBlockData>,
}

impl QueuedBlocks {
    pub(crate) fn enqueue(&mut self, new_block: SequencerBlockData) {
        let now = Instant::now();
        // checks if new block finalizes some block
        // (i) checks if new block is a child of any block
        if let Some(parent_id) = new_block.header.last_block_id {
            let parent_id: Vec<u8> = parent_id.hash.into();
            if let Some((parent, _)) = self.pending_finalization.remove(&parent_id) {
                self.finalized.push(parent);
            }
        }
        // (ii) checks if new block is parent to any block (finalizes itself)
        let mut new_block_is_finalized = false;
        let mut expired = Vec::new();
        for (block, insert_time) in self.pending_finalization.values() {
            if let Some(parent_id) = block.header.last_block_id {
                let parent_id: Vec<u8> = parent_id.hash.into();
                if new_block.block_hash == parent_id {
                    new_block_is_finalized = true;
                }
            }
            if now.saturating_duration_since(*insert_time) >= MAX_QUEUE_TIME {
                expired.push(block.block_hash.clone());
            }
        }

        if new_block_is_finalized {
            // insert new block into finalized queue
            self.finalized.push(new_block);
        } else {
            // insert new block into pending queue
            self.pending_finalization
                .insert(new_block.block_hash.clone(), (new_block, now));
        }

        // discards blocks that are taking too long to finalize
        for block_hash in expired {
            warn!(
                block_id = %Base64Display::new(&block_hash, &STANDARD),
                "discarding block that hasn't finalized in max queue time",
            );
            self.pending_finalization.remove(&block_hash);
        }
    }

    #[must_use]
    pub(crate) fn drain_finalized(&mut self) -> Vec<SequencerBlockData> {
        mem::take(&mut self.finalized)
    }

    pub(crate) fn has_finalized(&self) -> bool {
        !self.finalized.is_empty()
    }
}
