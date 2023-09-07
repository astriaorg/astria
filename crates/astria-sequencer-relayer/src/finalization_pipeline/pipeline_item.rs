use sequencer_types::SequencerBlockData;
use tendermint::Hash;

use super::{
    BlockWrapper,
    SoftBlock,
};

// tracks state of an item in the pipeline. all blocks received from the sequencer running this
// relayer are validated hence assumed to be heads, i.e. 1 block long forks of the canonical
// shared-sequencer chain
#[derive(Clone, Copy, Default, Debug)]
enum State {
    // head of 1 block long fork, block points to canonical head of chain. all blocks received
    // from sequencer are validated and hence assumed to be forks of canonical chain (single slot
    // finality).
    #[default]
    Head,
    // canonical head of chain, block points to final head of chain
    Soft,
}

/// Handles conversion between head block, soft block and final block as a block travels down the
/// pipeline.
#[derive(Debug)]
pub(crate) struct PipelineItem {
    block: BlockWrapper,
    state: State,
}

impl From<BlockWrapper> for PipelineItem {
    fn from(block: BlockWrapper) -> Self {
        Self {
            block,
            state: State::default(),
        }
    }
}

impl TryInto<SequencerBlockData> for PipelineItem {
    type Error = ();

    fn try_into(self) -> Result<SequencerBlockData, Self::Error> {
        self.block.try_into()
    }
}

impl PipelineItem {
    // pipeline item state changes:
    //
    // head -> soft (on soften)          i.e. fork -> canonical (on canonize)
    // soft -> final (on finalization)   i.e. canonical -> final (on finalization)

    // makes head block soft, i.e. makes fork block head of canonical chain
    #[must_use]
    pub(super) fn soften(mut self) -> Option<SoftBlock> {
        use State::*;
        let Self {
            state, ..
        } = self;
        match state {
            Head => {
                self.state = Soft;
                Some(SoftBlock {
                    block: self,
                })
            }
            Soft => None,
        }
    }

    // makes soft block final, i.e. finalizes the canonical head
    #[must_use]
    pub(super) fn finalize(self) -> Option<Result<SequencerBlockData, ()>> {
        use State::*;
        let Self {
            state,
            block,
        } = self;
        match state {
            Soft => Some(block.try_into()),
            Head => None,
        }
    }

    pub(super) fn block_hash(&self) -> Hash {
        self.block.block_hash()
    }

    pub(super) fn parent_block_hash(&self) -> Option<Hash> {
        self.block.parent_block_hash()
    }

    pub(super) fn height(&self) -> u64 {
        self.block.height()
    }
}
