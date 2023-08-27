use std::{
    collections::HashMap,
    mem,
};

use astria_sequencer_types::SequencerBlockData;
use tendermint::Hash;

pub(crate) mod head_candidate;
pub(crate) mod pipeline_item;

pub(crate) use head_candidate::HeadCandidate;
pub(crate) use pipeline_item::PipelineItem;

// head of the sequencer chain as observed on cometbft.
#[derive(Default)]
pub(crate) struct Head {
    block: PipelineItem,
}

// pipeline can handle forks 1 height deep
#[derive(Default)]
pub(crate) struct FinalizationPipeline {
    pub(crate) chain_head: Head,
    pending: HashMap<Hash, PipelineItem>,
    validator_finalized: Vec<SequencerBlockData>,
}

impl FinalizationPipeline {
    pub(crate) fn submit(&mut self, new_block: HeadCandidate) {
        let new_block: PipelineItem = new_block.into();

        match new_block.parent_block_hash() {
            Some(new_block_parent) => {
                debug_assert!(new_block.height() > self.chain_head.block.height());

                let steps = new_block.height() - self.chain_head.block.height();
                if steps == 1 {
                    debug_assert!(new_block_parent == self.chain_head.block.block_hash());

                    self.pending.insert(new_block.block_hash(), new_block);
                } else if steps == 2 {
                    let pending_at_prev_height = mem::replace(
                        &mut self.pending,
                        HashMap::from([(new_block.block_hash(), new_block)]),
                    );
                    for competing_block in pending_at_prev_height.into_values() {
                        debug_assert!(
                            competing_block.height() == self.chain_head.block.height() + 1
                        );

                        if competing_block.block_hash() == new_block_parent {
                            let old_head = mem::replace(
                                &mut self.chain_head,
                                competing_block.canonize().unwrap(),
                            );

                            if let Some(Ok(finalized_validator_block)) = old_head.block.finalize() {
                                self.validator_finalized.push(finalized_validator_block);
                            }
                            return;
                        }
                    }
                }
            }
            None => {
                // block is genesis
                self.chain_head = new_block.canonize().unwrap();
            }
        }
    }

    #[must_use]
    pub(crate) fn drain_finalized(&mut self) -> Vec<SequencerBlockData> {
        mem::take(&mut self.validator_finalized)
    }

    pub(crate) fn has_finalized(&self) -> bool {
        !self.validator_finalized.is_empty()
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
            Height,
            Id,
        },
        Hash,
    };

    use super::{
        FinalizationPipeline,
        HeadCandidate,
    };

    fn make_parent_and_child_blocks(
        parent_block_hash: u8,
        parent_block_height: u32,
        child_block_hash: u8,
        child_block_height: u32,
    ) -> [HeadCandidate; 2] {
        let mut parent_block = RawSequencerBlockData {
            block_hash: Hash::Sha256([parent_block_hash; 32]),
            ..Default::default()
        };
        parent_block.header.height = Height::from(parent_block_height);
        let parent_block = SequencerBlockData::from_raw_unverified(parent_block);

        let mut child_block = RawSequencerBlockData {
            block_hash: Hash::Sha256([child_block_hash; 32]),
            ..Default::default()
        };
        child_block.header.height = Height::from(child_block_height);
        let parent_id = Id {
            hash: parent_block.block_hash(),
            part_set_header: IdHeader::default(),
        };
        child_block.header.last_block_id = Some(parent_id.into());
        let child_block = SequencerBlockData::from_raw_unverified(child_block);

        let parent_block = HeadCandidate::ProposedByValidator(parent_block);
        let child_block = HeadCandidate::ProposedByValidator(child_block);

        [parent_block, child_block]
    }

    #[test]
    fn test_finalization_parent_is_genesis() {
        let [parent_block, child_block] = make_parent_and_child_blocks(0u8, 1, 1u8, 2);

        let [_, grandchild_block] = make_parent_and_child_blocks(1u8, 2, 2u8, 3);

        let mut pipeline = FinalizationPipeline::default();

        pipeline.submit(parent_block.clone());
        assert!(!pipeline.has_finalized());

        pipeline.submit(child_block);
        assert!(!pipeline.has_finalized());

        pipeline.submit(grandchild_block);
        assert!(pipeline.has_finalized());

        let mut finalized_blocks = pipeline.drain_finalized();

        assert_eq!(
            finalized_blocks.pop().unwrap(),
            parent_block.try_into().unwrap()
        )
    }

    #[test]
    fn test_finalization_three_competing_blocks_at_height_two() {
        let [parent_block, first_block] = make_parent_and_child_blocks(0u8, 1, 1u8, 2);

        let [_, second_block] = make_parent_and_child_blocks(0u8, 1, 2u8, 2);

        let [_, third_block] = make_parent_and_child_blocks(0u8, 1, 3u8, 2);

        let [_, child_second_block] = make_parent_and_child_blocks(2u8, 1, 4u8, 3);

        let mut pipeline = FinalizationPipeline::default();

        pipeline.submit(parent_block.clone());
        assert!(!pipeline.has_finalized());

        pipeline.submit(first_block);
        assert!(!pipeline.has_finalized());

        pipeline.submit(second_block);
        assert!(!pipeline.has_finalized());

        pipeline.submit(third_block);
        assert!(!pipeline.has_finalized());

        pipeline.submit(child_second_block);
        assert!(pipeline.has_finalized());

        let mut finalized_blocks = pipeline.drain_finalized();

        assert_eq!(finalized_blocks.len(), 1);
        assert_eq!(
            finalized_blocks.pop().unwrap(),
            parent_block.try_into().unwrap()
        );
    }
}
