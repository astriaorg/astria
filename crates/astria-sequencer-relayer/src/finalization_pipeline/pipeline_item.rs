use sequencer_types::SequencerBlockData;
use tendermint::Hash;

use super::SoftBlock;

// tracks state of an item in the pipeline. all blocks received from the sequencer running this
// relayer are validated hence assumed to be heads, i.e. 1 block long forks of the canonical
// shared-sequencer chain
#[derive(Default, Debug, Clone, Copy)]
enum CommitState {
    // head of 1 block long fork, block points to canonical head of chain. all blocks received
    // from sequencer are validated and hence assumed to be forks of canonical chain (single slot
    // finality).
    #[default]
    Head,
    // canonical head of chain, block points to final head of chain
    Soft,
}

// distinction is made between blocks published to cometbft by the sequencer running this relayer
// sidecar, and by other validator sequencer's. blocks published to cometbft by this sequencer,
// are published to DA by the relayer, hence stay in [`super::FinalizationPipeline`] until drained.
#[derive(Debug, Clone, Copy)]
enum ProposerState {
    LocalValidator,
    RemoteValidator,
}

/// Handles conversion between head block, soft block and final block as a block travels down the
/// pipeline.
#[derive(Debug, Clone)]
pub(super) struct PipelineItem {
    block: SequencerBlockData,
    commit_state: CommitState,
    proposer_state: ProposerState,
}

impl Into<SequencerBlockData> for PipelineItem {
    fn into(self) -> SequencerBlockData {
        self.block
    }
}

impl PipelineItem {
    pub(super) fn new_proposed_block(block: SequencerBlockData) -> Self {
        Self {
            block,
            commit_state: CommitState::default(),
            proposer_state: ProposerState::LocalValidator,
        }
    }

    pub(super) fn new_remote_block(block: SequencerBlockData) -> Self {
        Self {
            block,
            commit_state: CommitState::default(),
            proposer_state: ProposerState::RemoteValidator,
        }
    }

    // pipeline item commit state changes:
    //
    // head -> soft (on soften)          i.e. fork -> canonical (on canonize)
    // soft -> final (on finalization)   i.e. canonical -> final (on finalization)

    // makes head block soft, i.e. makes fork block head of canonical chain
    #[must_use]
    pub(super) fn soften(mut self) -> Option<SoftBlock> {
        use CommitState::*;
        let Self {
            commit_state: state,
            ..
        } = self;
        match state {
            Head => {
                self.commit_state = Soft;
                Some(SoftBlock {
                    block: self,
                })
            }
            Soft => None,
        }
    }

    // makes soft block final, i.e. finalizes the canonical head
    #[must_use]
    pub(super) fn finalize(self) -> Option<SequencerBlockData> {
        use CommitState::*;
        use ProposerState::*;
        let Self {
            commit_state,
            block,
            proposer_state,
        } = self;
        match proposer_state {
            LocalValidator => match commit_state {
                Soft => Some(block), // finalizes, returned for store till pipeline drained
                Head => None,
            },
            RemoteValidator => None, // finalizes and is discarded
        }
    }

    pub(super) fn block_hash(&self) -> Hash {
        self.block.block_hash()
    }

    pub(super) fn parent_block_hash(&self) -> Option<Hash> {
        self.block.parent_block_hash()
    }

    pub(super) fn height(&self) -> u64 {
        self.block.header().height.into()
    }
}
