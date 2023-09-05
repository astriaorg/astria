use astria_sequencer_types::SequencerBlockData;
use tendermint::Hash;
use tendermint_rpc::endpoint::block;

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

    pub(crate) fn new_by_other_validator(block: block::Response) -> Self {
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

impl From<block::Response> for SequencerBlockSubset {
    fn from(res: block::Response) -> Self {
        let block_hash = res.block_id.hash;
        let parent_block_hash = res.block.header.last_block_id.map(|id| id.hash);
        let height = res.block.header.height.into();
        Self {
            block_hash,
            parent_block_hash,
            height,
        }
    }
}
