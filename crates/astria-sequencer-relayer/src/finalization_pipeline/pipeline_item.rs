use astria_sequencer_types::SequencerBlockData;
use tendermint::Hash;

use super::{
    Head,
    HeadCandidate,
};

#[derive(Clone, Copy, Default, Debug)]
enum State {
    #[default]
    Fork,
    Canonical,
}

#[derive(Default, Debug)]
pub(crate) struct PipelineItem {
    block: HeadCandidate,
    state: State,
}

impl From<HeadCandidate> for PipelineItem {
    fn from(block: HeadCandidate) -> Self {
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
    // head candidate state changes:
    //
    // fork -> canonical (on canonize)

    // integrates a block into the canonical chain
    #[must_use]
    pub(super) fn canonize(mut self) -> Option<Head> {
        use State::*;
        let Self {
            state, ..
        } = self;
        match state {
            Fork => {
                self.state = Canonical;
                Some(Head {
                    block: self,
                })
            }
            Canonical => None,
        }
    }

    // finalizes a block at HEAD^
    #[must_use]
    pub(super) fn finalize(self) -> Option<Result<SequencerBlockData, ()>> {
        use State::*;
        let Self {
            state,
            block,
        } = self;
        match state {
            Canonical => Some(block.try_into()),
            Fork => None,
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
