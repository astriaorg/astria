use std::{
    collections::HashMap,
    time::{
        Duration,
        Instant,
    },
};

use base64::{
    display::Base64Display,
    engine::general_purpose::STANDARD,
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
        let now = std::time::Instant::now();
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
        for (block, _) in self.pending_finalization.values() {
            if let Some(parent_id) = block.header.last_block_id {
                let parent_id: Vec<u8> = parent_id.hash.into();
                if new_block.block_hash == parent_id {
                    new_block_is_finalized = true;
                }
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

        self.discard_timed_out(now);
    }

    fn discard_timed_out(&mut self, current_time: Instant) {
        // discards blocks that are taking too long to finalize
        self.pending_finalization
            .retain(|block_id, (_, insert_time)| {
                if current_time.saturating_duration_since(*insert_time) < MAX_QUEUE_TIME {
                    true
                } else {
                    warn!(
                        block_id = %Base64Display::new(block_id, &STANDARD),
                        "discarding block that hasn't finalized in max queue time",
                    );
                    false
                }
            });
    }

    #[must_use]
    pub(crate) fn drain_finalized(&mut self) -> Vec<SequencerBlockData> {
        self.finalized.drain(..).collect()
    }

    pub(crate) fn finalized_is_empty(&self) -> bool {
        self.finalized.is_empty()
    }
}
