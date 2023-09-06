use astria_sequencer_types::SequencerBlockData;
use tendermint::Hash;

/// Wrapper for sending a sequencer block down the finalization pipeline. A distinction is made
/// between blocks published to cometbft by the sequencer running this relayer sidecar, and
/// by other sequencer's. Blocks published to cometbft by this sequencer, should be published to
/// DA by the relayer, hence they end up in the `finalized` queue in
/// [`super::FinalizationPipeline`].
#[derive(Clone, Default, Debug)]
pub(crate) enum BlockWrapper {
    /// Blocks proposed by the validator running this relayer.
    FromValidator(SequencerBlockData),
    /// Blocks proposed by other validators, received by the sequencer over cometbft.
    FromOtherValidator(SequencerBlockSubset),
    #[default]
    Default,
}

impl TryInto<SequencerBlockData> for BlockWrapper {
    type Error = ();

    fn try_into(self) -> Result<SequencerBlockData, Self::Error> {
        use BlockWrapper::*;
        match self {
            FromValidator(block) => Ok(block),
            FromOtherValidator(_) => Err(()),
            _ => unreachable!(),
        }
    }
}

impl BlockWrapper {
    pub(crate) fn new_by_validator(block: SequencerBlockData) -> Self {
        Self::FromValidator(block)
    }

    pub(crate) fn new_by_other_validator(block: SequencerBlockData) -> Self {
        Self::FromOtherValidator(block.into())
    }

    pub(crate) fn block_hash(&self) -> Hash {
        use BlockWrapper::*;
        match self {
            FromValidator(block) => block.block_hash(),
            FromOtherValidator(block) => block.block_hash,
            _ => unreachable!(),
        }
    }

    pub(crate) fn parent_block_hash(&self) -> Option<Hash> {
        use BlockWrapper::*;
        match self {
            FromValidator(block) => block.parent_block_hash(),
            FromOtherValidator(block) => block.parent_block_hash,
            _ => unreachable!(),
        }
    }

    pub(crate) fn height(&self) -> u64 {
        use BlockWrapper::*;
        match self {
            FromValidator(block) => block.header().height.into(),
            FromOtherValidator(block) => block.height,
            _ => unreachable!(),
        }
    }
}

/// Subset of the set of data held by [`SequencerBlockData`] needed to verify canonicity of chain.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SequencerBlockSubset {
    block_hash: Hash,
    parent_block_hash: Option<Hash>,
    height: u64,
}

impl From<SequencerBlockData> for SequencerBlockSubset {
    fn from(block: SequencerBlockData) -> Self {
        let block_hash = block.block_hash();
        let parent_block_hash = block.parent_block_hash();
        let height = block.header().height.into();
        Self {
            block_hash,
            parent_block_hash,
            height,
        }
    }
}
