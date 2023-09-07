use std::{
    collections::HashMap,
    mem,
};

use sequencer_types::SequencerBlockData;
use tendermint::Hash;

pub(crate) mod block_wrapper;
pub(crate) mod pipeline_item;

pub(crate) use block_wrapper::BlockWrapper;
pub(crate) use pipeline_item::PipelineItem;

// the height of soft block + 1, i.e. the height of the canonical head + 1, child height with
// respect to soft block
const HEAD_HEIGHT: u64 = 1;
// the height of soft block + 2, i.e. the height of the canonical head + 2, grandchild height with
// respect to soft block
const HEAD_PLUS_ONE_HEIGHT: u64 = 2;

/// Tracking canonical head of shared-sequencer chain as observed on cometbft.
pub(crate) struct SoftBlock {
    block: PipelineItem,
}

/// Pipeline handles validated blocks according to single slot finality (cometbft), received in
/// sequential order with respect to height, i.e. pipeline handles forks 1 block long.
///
/// (warning! pipeline not intended for rpc connection relayer-sequencer on other IF than
/// localhost, not designed for unordered arrival of blocks over network)
///
/// Fork choice is such that, the block pointed to by the FCFS block at grandchild height relative
/// to the soft block, i.e. to the canonical head of the shared-sequencer chain, is the new soft
/// block, i.e. the new shared-sequencer chain canonical head (single slot finality, also fast
/// finality, instant finality).
///
/// Fork choice is executed when a block at grandchild height relative to the canonical head is
/// received. Blocks are assumed to come (from cometbft) via sequencer to the relayer in
/// sequential order with respect to height and to be validated. Assuming the validator set is
/// honest, the conversion from tendermint block to [`SequencerBlockData`] will be successful and
/// thereby also submission to the pipeline (validators check if commitment to rollup data is
/// correct <https://github.com/astriaorg/astria/blob/main/specs/sequencer-app.md#processproposal>). This means all blocks at head height can be assumed to point to the soft
/// block, i.e. all blocks at canonical head height + 1 point to the canonical head (blocks at
/// grandchild height relative to canonical head point to a fork of the canonical shared-sequencer
/// chain). As a follow, the arrival of a child block to any head block is expected to finalize
/// that head block.
#[derive(Default)]
pub(crate) struct FinalizationPipeline {
    /// Head of the canonical shared-sequencer chain.
    soft_block: Option<SoftBlock>,
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
                let soft_block = self.soft_block.as_ref().expect("should post genesis");

                debug_assert!(new_block.height() > soft_block.block.height());

                let steps = new_block.height() - soft_block.block.height();
                if steps == HEAD_HEIGHT {
                    // finalization pipeline assumes blocks arrive in sequential order from
                    // sequencer and are validated
                    debug_assert!(parent_of_new_block == soft_block.block.block_hash());

                    self.pending.insert(new_block.block_hash(), new_block);
                } else if steps == HEAD_PLUS_ONE_HEIGHT {
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
                                Some(
                                    competing_block
                                        .soften()
                                        .expect("should be first attempt to soften block"),
                                ),
                            );

                            if let Some(Ok(finalized_validator_block)) =
                                old_head.expect("should be post genesis").block.finalize()
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
    use sequencer_types::test_utils;
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
        parent_block_hash: Option<Hash>,
        parent_block_height: u32,
        child_block_height: u32,
        parent_block_salt: u8,
        child_block_salt: u8,
    ) -> [BlockWrapper; 2] {
        let mut parent_block = test_utils::new_raw_block(parent_block_salt);
        parent_block.header.height = Height::from(parent_block_height);
        let parent_block = test_utils::from_raw(parent_block);

        let mut child_block = test_utils::new_raw_block(child_block_salt);
        child_block.header.height = Height::from(child_block_height);
        let parent_id = Id {
            hash: parent_block_hash
                .or_else(|| Some(parent_block.block_hash()))
                .unwrap(),
            part_set_header: IdHeader::default(),
        };
        child_block.header.last_block_id = Some(parent_id.into());
        let child_block = test_utils::from_raw(child_block);

        let parent_block = BlockWrapper::FromValidator(parent_block);
        let child_block = BlockWrapper::FromValidator(child_block);

        [parent_block, child_block]
    }

    #[test]
    fn finalization_parent_is_genesis() {
        let [genesis_block, child_block] = make_parent_and_child_blocks(None, 1, 2, 0u8, 1u8);

        let child_block_hash = child_block.block_hash();

        let [_, grandchild_block] =
            make_parent_and_child_blocks(Some(child_block_hash), 2, 3, 1u8, 2u8);

        let mut pipeline = FinalizationPipeline::default();

        pipeline.submit(genesis_block.clone());
        assert!(!pipeline.has_finalized());

        pipeline.submit(child_block);
        assert!(!pipeline.has_finalized());

        pipeline.submit(grandchild_block);
        assert!(pipeline.has_finalized());

        let mut finalized_blocks = pipeline.drain_finalized();

        assert_eq!(
            finalized_blocks.pop().unwrap(),
            genesis_block.try_into().unwrap()
        )
    }

    #[test]
    fn finalization_three_competing_blocks_at_height_two() {
        let [genesis_block, first_block] = make_parent_and_child_blocks(None, 1, 2, 0u8, 1u8);

        let genesis_block_hash = genesis_block.block_hash();

        let [_, second_block] =
            make_parent_and_child_blocks(Some(genesis_block_hash), 1, 2, 0u8, 2u8);
        let [_, third_block] =
            make_parent_and_child_blocks(Some(genesis_block_hash), 1, 2, 0u8, 3u8);

        let second_block_hash = second_block.block_hash();

        let [_, child_second_block] =
            make_parent_and_child_blocks(Some(second_block_hash), 1, 3, 2u8, 4u8);

        let mut pipeline = FinalizationPipeline::default();

        pipeline.submit(genesis_block.clone()); // height 1
        assert!(!pipeline.has_finalized());

        pipeline.submit(first_block); // height 2
        assert!(!pipeline.has_finalized());

        pipeline.submit(second_block); // height 2
        assert!(!pipeline.has_finalized());

        pipeline.submit(third_block); // height 2
        assert!(!pipeline.has_finalized());

        pipeline.submit(child_second_block); // height 3, finalizes second block
        assert!(pipeline.has_finalized());

        let mut finalized_blocks = pipeline.drain_finalized();

        assert_eq!(finalized_blocks.len(), 1);
        assert_eq!(
            finalized_blocks.pop().unwrap(),
            genesis_block.try_into().unwrap()
        );
    }
}
