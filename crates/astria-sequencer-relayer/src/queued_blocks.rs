use std::{
    collections::HashMap,
    mem,
};

use astria_sequencer_types::SequencerBlockData;
use tendermint::Hash;
use tokio::time::{
    Duration,
    Instant,
};
use tracing::warn;

use crate::config::MAX_RELAYER_QUEUE_TIME_MS;

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
    pending: HashMap<Hash, QueueItem>,
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
        parent_block_hash: Hash,
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
                                parent_block_hash,
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

    fn parent_block_hash(&self) -> Option<Hash> {
        use QueueItem::*;
        match self {
            Finalized {
                parent_block_hash, ..
            } => Some(*parent_block_hash),
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
                if Some(new_block.block_hash()) == child.parent_block_hash() {
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
                        new_block.block_hash(),
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
                new_block.block_hash(),
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
                block_id = %block_hash,
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
    use astria_sequencer_types::{
        RawSequencerBlockData,
        SequencerBlockData,
    };
    use tendermint::{
        block::{
            parts::Header as IdHeader,
            Id,
        },
        Hash,
    };

    use super::QueuedBlocks;

    fn make_parent_and_child_blocks(
        parent_block_hash: u8,
        child_block_hash: u8,
    ) -> [SequencerBlockData; 2] {
        let parent_block = RawSequencerBlockData {
            block_hash: Hash::Sha256([parent_block_hash; 32]),
            ..Default::default()
        };
        let parent_block = SequencerBlockData::from_raw_unverified(parent_block);

        let mut child_block = RawSequencerBlockData {
            block_hash: Hash::Sha256([child_block_hash; 32]),
            ..Default::default()
        };
        let parent_id = Id {
            hash: parent_block.block_hash(),
            part_set_header: IdHeader::default(),
        };
        child_block.header.last_block_id = Some(parent_id.into());
        let child_block = SequencerBlockData::from_raw_unverified(child_block);

        [parent_block, child_block]
    }

    #[test]
    fn test_finalization_parent_is_genesis_and_queued_before_child() {
        let [parent_block, child_block] = make_parent_and_child_blocks(0, 1);

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
        let [parent_block, child_block] = make_parent_and_child_blocks(0, 1);

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
        let [grandparent_block, parent_block] = make_parent_and_child_blocks(0, 1);

        let [_, child_block] = make_parent_and_child_blocks(1, 2);

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
        let [grandparent_block, parent_block] = make_parent_and_child_blocks(0, 1);

        let [_, child_block] = make_parent_and_child_blocks(1, 2);

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
        let [grandparent_block, parent_block] = make_parent_and_child_blocks(0, 1);

        let [_, child_block] = make_parent_and_child_blocks(1, 2);

        let [_, grandchild_block] = make_parent_and_child_blocks(2, 3);

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
        let [grandparent_block, parent_block] = make_parent_and_child_blocks(0, 1);

        let [_, child_block] = make_parent_and_child_blocks(1, 2);

        let [_, grandchild_block] = make_parent_and_child_blocks(2, 3);

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
