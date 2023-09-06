use std::{
    collections::HashMap,
    mem,
};

use astria_sequencer_types::SequencerBlockData;
use tendermint::Hash;

pub(crate) mod block_wrapper;
pub(crate) mod pipeline_item;

pub(crate) use block_wrapper::BlockWrapper;
pub(crate) use pipeline_item::PipelineItem;

const CHILD_HEIGHT: u64 = 1;
const GRANDCHILD_HEIGHT: u64 = 2;

/// Tracking canonical head of shared-sequencer chain as observed on cometbft.
pub(crate) struct SoftBlock {
    block: PipelineItem,
}

/// Pipeline handles validated blocks received from sequencer on localhost interface, as according
/// to cometbft single slot finality, i.e. pipeline handles forks 1 block long.
///
/// (warning! pipeline not intended for rpc connection relayer-sequencer on other IF than localhost,
/// not designed for unordered arrival of blocks over network)
///
/// Fork choice is such that, the block pointed to by the FCFS block at grandchild height relative
/// to the soft block, i.e. to the canonical head of the shared-sequencer chain, is the
/// new soft block, i.e. the new shared-sequencer chain canonical head.
///
/// Fork choice is executed when a block at grandchild height relative to the
/// canonical head is received. Blocks are assumed to come (from cometbft) via sequencer in
/// sequential order, pointing to the canonical chain (blocks at child height relative to
/// canonical head point to canonical head, and blocks at grandchild height relative to canonical
/// head point to a fork of the canonical chain).
#[derive(Default)]
pub(crate) struct FinalizationPipeline {
    /// Head of the canonical chain.
    pub(crate) soft_block: Option<SoftBlock>,
    // queue of blocks pending finalization (xor pending orphanhood)
    pending: HashMap<Hash, PipelineItem>,
    // blocks proposed by the sequencer running this relayer. to be submitted to DA layer.
    finalized: Vec<SequencerBlockData>,
}

impl FinalizationPipeline {
    pub(crate) fn submit(&mut self, new_block: BlockWrapper) {
        let new_block: PipelineItem = new_block.into();

        match new_block.parent_block_hash() {
            Some(parent_of_new_block) => {
                let soft_block = self.soft_block.as_ref().expect("post genesis");

                debug_assert!(new_block.height() > soft_block.block.height());

                let steps = new_block.height() - soft_block.block.height();
                if steps == CHILD_HEIGHT {
                    // finalization pipeline assumes blocks arrive in sequential order from
                    // sequencer
                    debug_assert!(parent_of_new_block == soft_block.block.block_hash());

                    self.pending.insert(new_block.block_hash(), new_block);
                } else if steps == GRANDCHILD_HEIGHT {
                    // do fork choice
                    let pending_at_prev_height = mem::replace(
                        &mut self.pending,
                        HashMap::from([(new_block.block_hash(), new_block)]),
                    );
                    for competing_block in pending_at_prev_height.into_values() {
                        debug_assert!(competing_block.height() == soft_block.block.height() + 1);

                        if competing_block.block_hash() == parent_of_new_block {
                            let old_head = mem::replace(
                                &mut self.soft_block,
                                Some(competing_block.soften().expect(
                                    "all blocks from sequencer are canonical (single slot \
                                     finality)",
                                )),
                            );

                            if let Some(Ok(finalized_validator_block)) =
                                old_head.expect("post genesis").block.finalize()
                            {
                                self.finalized.push(finalized_validator_block);
                            }
                            return;
                        }
                    }
                }
            }
            None => {
                // block is genesis
                self.soft_block = new_block.soften();
            }
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
            Height,
            Id,
        },
        Hash,
    };

    use super::{
        BlockWrapper,
        FinalizationPipeline,
    };

    fn make_parent_and_child_blocks(
        parent_block_hash: u8,
        parent_block_height: u32,
        child_block_hash: u8,
        child_block_height: u32,
    ) -> [BlockWrapper; 2] {
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

        let parent_block = BlockWrapper::FromValidator(parent_block);
        let child_block = BlockWrapper::FromValidator(child_block);

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

        pipeline.submit(child_second_block); // finalizes second block
        assert!(pipeline.has_finalized());

        let mut finalized_blocks = pipeline.drain_finalized();

        assert_eq!(finalized_blocks.len(), 1);
        assert_eq!(
            finalized_blocks.pop().unwrap(),
            parent_block.try_into().unwrap()
        );
    }
}
