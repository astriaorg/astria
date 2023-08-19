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

use crate::{
    config::MAX_RELAYER_QUEUE_TIME_MS,
    types::SequencerBlockData,
};

const MAX_QUEUE_TIME: Duration = Duration::from_millis(MAX_RELAYER_QUEUE_TIME_MS);

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
            if !new_block_is_finalized {
                if let Some(parent_id) = block.header.last_block_id {
                    let parent_id: Vec<u8> = parent_id.hash.into();
                    if new_block.block_hash == parent_id {
                        new_block_is_finalized = true;
                        continue;
                    }
                }
            }
            // optimistic algorithm, time out check is done after checking finalization
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

#[cfg(test)]
mod test {
    use tendermint::{
        block::{
            parts::Header as IdHeader,
            Id,
        },
        Hash,
    };

    use super::QueuedBlocks;
    use crate::types::SequencerBlockData;

    type BlockHash = [u8; 32];

    fn make_parent_and_child_blocks(
        parent_block_hash: BlockHash,
        child_block_hash: BlockHash,
    ) -> [SequencerBlockData; 2] {
        let parent_block = SequencerBlockData {
            block_hash: parent_block_hash.to_vec(),
            ..Default::default()
        };

        let parent_id = Id {
            hash: Hash::try_from(parent_block.block_hash.clone()).unwrap(),
            part_set_header: IdHeader::default(),
        };
        let mut child_block = SequencerBlockData {
            block_hash: child_block_hash.to_vec(),
            ..Default::default()
        };
        child_block.header.last_block_id = Some(parent_id);

        [parent_block, child_block]
    }

    #[test]
    fn test_finalization_parent_block_is_genesis_and_queued_before_child() {
        let [parent_block, child_block] = make_parent_and_child_blocks([0u8; 32], [1u8; 32]);

        let mut queue = QueuedBlocks::default();

        queue.enqueue(parent_block.clone());
        assert!(!queue.has_finalized());

        queue.enqueue(child_block);
        assert!(queue.has_finalized());

        let finalized_blocks = queue.drain_finalized();
        assert_eq!(finalized_blocks, vec!(parent_block))
    }

    #[test]
    fn test_finalization_parent_block_is_genesis_and_queued_after_child() {
        let [parent_block, child_block] = make_parent_and_child_blocks([0u8; 32], [1u8; 32]);

        let mut queue = QueuedBlocks::default();

        queue.enqueue(child_block);
        assert!(!queue.has_finalized());

        queue.enqueue(parent_block.clone());
        assert!(queue.has_finalized());

        let finalized_blocks = queue.drain_finalized();
        assert_eq!(finalized_blocks, vec!(parent_block))
    }

    #[test]
    fn test_finalization_grand_parent_block_queued_before_parent() {
        let [grandparent_block, parent_block] = make_parent_and_child_blocks([0u8; 32], [1u8; 32]);

        let [_, child_block] = make_parent_and_child_blocks([1u8; 32], [2u8; 32]);

        let mut queue = QueuedBlocks::default();

        queue.enqueue(grandparent_block.clone());
        assert!(!queue.has_finalized());

        queue.enqueue(parent_block.clone());
        assert!(queue.has_finalized());

        queue.enqueue(child_block);
        assert!(queue.has_finalized());

        let finalized_blocks = queue.drain_finalized();
        assert_eq!(finalized_blocks.len(), 2);
        assert!(finalized_blocks.contains(&grandparent_block));
        assert!(finalized_blocks.contains(&parent_block));
    }

    #[test]
    fn test_finalization_grand_parent_block_queued_after_child() {
        let [grandparent_block, parent_block] = make_parent_and_child_blocks([0u8; 32], [1u8; 32]);

        let [_, child_block] = make_parent_and_child_blocks([1u8; 32], [2u8; 32]);

        let mut queue = QueuedBlocks::default();

        queue.enqueue(child_block);
        assert!(!queue.has_finalized());

        queue.enqueue(grandparent_block.clone());
        assert!(!queue.has_finalized());

        queue.enqueue(parent_block.clone());
        assert!(queue.has_finalized());

        let finalized_blocks = queue.drain_finalized();
        assert_eq!(finalized_blocks.len(), 2);
        assert!(finalized_blocks.contains(&grandparent_block));
        assert!(finalized_blocks.contains(&parent_block));
    }
}
