use astria_sequencer_types::SequencerBlockData;
use tendermint::Hash;

use super::{
    HeadBlock,
    SoftBlock,
};

#[derive(Clone, Copy, Default, Debug)]
enum State {
    #[default]
    Head, // head of 1 block long fork, block points to canonical head of chain
    Soft, // head of canonical chain, block points to final head of chain
}

#[derive(Default, Debug)]
pub(crate) struct PipelineItem {
    block: HeadBlock,
    state: State,
}

impl From<HeadBlock> for PipelineItem {
    fn from(block: HeadBlock) -> Self {
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
    // head -> soft (on soften)                         i.e. fork -> canonical (on canonize)
    // soft -> final (on finalization)                  i.e. canonical -> final (on finalization)

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
