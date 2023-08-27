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

// rn, current height is read from sequencer. only blocks of height higher than current height
// will be converted to sequencer data blocks by relayer (fn
// convert_block_response_to_sequencer_block_data in ../src/relayer.rs). the sequencer is only
// polled for one block per sequencer tick (fn handle_sequencer_tick in ../src/relayer.rs). hence
// blocks will always come sequentially to queue unless sequencer would allow for forks.
// nonetheless, this queue is future proof in the sense that it can handle arrival of blocks in
// non-sequential order on a permissioned overlay. todo(emhane): loosen restrictions on converting
// tendermint blocks based on height.
#[derive(Default)]
pub(crate) struct QueuedBlocks {
    pending: HashMap<Vec<u8>, QueueItem>,
    finalized: Vec<SequencerBlockData>,
}

#[derive(Debug)]
enum PendingCanonicalState {
    Canonical, // has finalized a parent
    NotCanonical,
}

#[derive(Debug)]
enum QueueItem {
    PendingFinalization {
        block: SequencerBlockData,
        insert_time: Instant,
        canonical: PendingCanonicalState,
    },
    Finalized {
        parent_block_hash: Vec<u8>,
        insert_time: Instant,
    }, // has been finalized by a child
    FinalizedCanonical, // safe to remove
}

impl QueueItem {
    // state changes:
    //
    // pending -> finalized (on finalization) | pending canonical (on canonization)
    // pending-canonical -> finalized-canonical (on finalization)
    // finalized -> finalized-canonical (on canonization)
    // finalized-canonical -> (remove)

    // returns the finalized block
    fn finalize(&mut self) -> Option<SequencerBlockData> {
        use PendingCanonicalState::*;
        use QueueItem::*;
        match self {
            PendingFinalization {
                block,
                insert_time,
                canonical,
            } => {
                let updated_item = match canonical {
                    Canonical => FinalizedCanonical,
                    NotCanonical => {
                        if let Some(parent_block_hash) = block.parent_block_hash() {
                            Finalized {
                                parent_block_hash: parent_block_hash.clone(),
                                insert_time: *insert_time,
                            }
                        } else {
                            FinalizedCanonical // genesis block is automatically canonical
                        }
                    }
                };

                if let PendingFinalization {
                    block, ..
                } = mem::replace(self, updated_item)
                {
                    Some(block)
                } else {
                    unreachable!()
                }
            }
            Finalized {
                ..
            }
            | FinalizedCanonical => None, // already finalized, uncle blocks not acknowledged
        }
    }

    // marks queue item as canonical, meaning it is safe to remove the block on finalization
    fn canonize(&mut self) {
        use PendingCanonicalState::*;
        use QueueItem::*;
        match self {
            PendingFinalization {
                canonical, ..
            } => {
                match canonical {
                    Canonical => {} // already canonical, uncle blocks not acknowledged
                    NotCanonical => *canonical = Canonical,
                }
            }
            Finalized {
                ..
            } => {
                *self = FinalizedCanonical;
            }
            FinalizedCanonical => {} // already canonical, uncle blocks not acknowledged
        }
    }

    fn parent_block_hash(&self) -> Option<Vec<u8>> {
        use QueueItem::*;
        match self {
            Finalized {
                parent_block_hash, ..
            } => Some(parent_block_hash.clone()),
            PendingFinalization {
                block, ..
            } => block.parent_block_hash(),
            FinalizedCanonical => None,
        }
    }

    fn is_canonical(&self) -> bool {
        use PendingCanonicalState::*;
        use QueueItem::*;
        match self {
            PendingFinalization {
                canonical, ..
            } => match canonical {
                Canonical => true,
                NotCanonical => false,
            },
            Finalized {
                ..
            } => false,
            FinalizedCanonical => true,
        }
    }

    fn insert_time(&self) -> Option<Instant> {
        use QueueItem::*;
        match self {
            PendingFinalization {
                insert_time, ..
            }
            | Finalized {
                insert_time, ..
            } => Some(*insert_time),
            FinalizedCanonical => None,
        }
    }
}

impl QueuedBlocks {
    pub(crate) fn enqueue(&mut self, new_block: SequencerBlockData) {
        use QueueItem::*;
        let now = Instant::now();
        // checks if new block finalizes some block
        // (i) checks if new block is a child of any block. checks its parent block hash.
        let mut new_block_is_canonical = false;
        if let Some(parent_block_hash) = new_block.parent_block_hash() {
            if let Some(parent) = self.pending.get_mut(&parent_block_hash.clone()) {
                // update parent queue item
                if let Some(finalized_parent) = parent.finalize() {
                    self.finalized.push(finalized_parent);
                    new_block_is_canonical = true;
                }
            }
        }
        // (ii) checks if new block is parent to any block (finalizes itself). checks parent block
        // hash of non-canonical blocks.
        let mut new_block_is_finalized = false;
        let mut expired = Vec::new();
        for (child_block_hash, child) in self.pending.iter_mut() {
            // disregard uncle blocks, shouldn't exist
            if !new_block_is_finalized && !child.is_canonical() {
                if Some(&new_block.block_hash) == child.parent_block_hash().as_ref() {
                    child.canonize(); // update state of queue item
                    new_block_is_finalized = true;
                    continue;
                }
            }

            // optimistic algorithm, time out check is done after checking finalization triggered
            // by new block

            let insert_time = child.insert_time();
            if insert_time.is_none()
                || now.saturating_duration_since(insert_time.unwrap()) >= MAX_QUEUE_TIME
            {
                expired.push(child_block_hash.clone())
            }
        }

        if new_block_is_finalized {
            if !new_block_is_canonical {
                // remember block so its parent can finalize if it didn't arrive yet, unless block
                // is genesis
                if let Some(parent_block_hash) = new_block.parent_block_hash() {
                    self.pending.insert(
                        new_block.block_hash.clone(),
                        Finalized {
                            parent_block_hash: parent_block_hash.clone(),
                            insert_time: now,
                        },
                    );
                }
            }
            // insert new block into finalized queue
            self.finalized.push(new_block);
        } else {
            use PendingCanonicalState::*;
            // insert new block into pending queue
            self.pending.insert(
                new_block.block_hash.clone(),
                PendingFinalization {
                    block: new_block,
                    insert_time: now,
                    canonical: if new_block_is_canonical {
                        Canonical
                    } else {
                        NotCanonical
                    },
                },
            );
        }

        // discards blocks that are taking too long to finalize
        for block_hash in expired {
            warn!(
                block_id = %Base64Display::new(&block_hash, &STANDARD),
                "discarding block not finalized or part of canonical chain in max queue time",
            );
            self.pending.remove(&block_hash);
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
    fn test_finalization_parent_is_genesis_and_queued_before_child() {
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
    fn test_finalization_parent_is_genesis_and_queued_after_child() {
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
    fn test_finalization_grandparent_queued_before_parent() {
        let [grandparent_block, parent_block] = make_parent_and_child_blocks([0u8; 32], [1u8; 32]);

        let [_, child_block] = make_parent_and_child_blocks([1u8; 32], [2u8; 32]);

        let mut queue = QueuedBlocks::default();

        queue.enqueue(grandparent_block.clone());
        assert!(!queue.has_finalized());

        queue.enqueue(parent_block.clone()); // finalizes grandparent
        assert!(queue.has_finalized());

        queue.enqueue(child_block); // finalizes parent
        assert!(queue.has_finalized());

        let mut finalized_blocks = queue.drain_finalized();

        assert_eq!(finalized_blocks.len(), 2);
        assert_eq!(finalized_blocks.pop().unwrap(), parent_block);
        assert_eq!(finalized_blocks.pop().unwrap(), grandparent_block);
    }

    #[test]
    fn test_finalization_grandparent_block_queued_after_child() {
        let [grandparent_block, parent_block] = make_parent_and_child_blocks([0u8; 32], [1u8; 32]);

        let [_, child_block] = make_parent_and_child_blocks([1u8; 32], [2u8; 32]);

        let mut queue = QueuedBlocks::default();

        queue.enqueue(child_block);
        assert!(!queue.has_finalized());

        queue.enqueue(grandparent_block.clone());
        assert!(!queue.has_finalized());

        queue.enqueue(parent_block.clone()); // finalizes grandparent and itself
        assert!(queue.has_finalized());

        let mut finalized_blocks = queue.drain_finalized();

        assert_eq!(finalized_blocks.len(), 2);
        assert_eq!(finalized_blocks.pop().unwrap(), parent_block);
        assert_eq!(finalized_blocks.pop().unwrap(), grandparent_block);
    }

    #[test]
    fn test_finalization_four_links_child_queued_after_grandchild() {
        let [grandparent_block, parent_block] = make_parent_and_child_blocks([0u8; 32], [1u8; 32]);

        let [_, child_block] = make_parent_and_child_blocks([1u8; 32], [2u8; 32]);

        let [_, grandchild_block] = make_parent_and_child_blocks([2u8; 32], [3u8; 32]);

        let mut queue = QueuedBlocks::default();

        queue.enqueue(grandparent_block.clone());
        assert!(!queue.has_finalized());

        queue.enqueue(grandchild_block);
        assert!(!queue.has_finalized());

        queue.enqueue(child_block.clone()); // finalizes itself
        assert!(queue.has_finalized());

        queue.enqueue(parent_block.clone()); // finalizes grandparent and itself
        assert!(queue.has_finalized());

        let mut finalized_blocks = queue.drain_finalized();

        assert_eq!(finalized_blocks.len(), 3);
        assert_eq!(finalized_blocks.pop().unwrap(), parent_block);
        assert_eq!(finalized_blocks.pop().unwrap(), grandparent_block);
        assert_eq!(finalized_blocks.pop().unwrap(), child_block);
    }

    #[test]
    fn test_finalization_four_links_child_queued_before_grandchild() {
        let [grandparent_block, parent_block] = make_parent_and_child_blocks([0u8; 32], [1u8; 32]);

        let [_, child_block] = make_parent_and_child_blocks([1u8; 32], [2u8; 32]);

        let [_, grandchild_block] = make_parent_and_child_blocks([2u8; 32], [3u8; 32]);

        let mut queue = QueuedBlocks::default();

        queue.enqueue(grandparent_block.clone());
        assert!(!queue.has_finalized());

        queue.enqueue(child_block.clone());
        assert!(!queue.has_finalized());

        queue.enqueue(grandchild_block); // finalizes child
        assert!(queue.has_finalized());

        queue.enqueue(parent_block.clone()); // finalizes grandparent and itself
        assert!(queue.has_finalized());

        let mut finalized_blocks = queue.drain_finalized();

        assert_eq!(finalized_blocks.len(), 3);
        assert_eq!(finalized_blocks.pop().unwrap(), parent_block);
        assert_eq!(finalized_blocks.pop().unwrap(), grandparent_block);
        assert_eq!(finalized_blocks.pop().unwrap(), child_block);
    }
}
