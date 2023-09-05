use astria_sequencer_types::SequencerBlockData;
use tendermint::Hash;
use tendermint_rpc::endpoint::block;

#[derive(Clone, Default, Debug)]
pub(crate) enum HeadBlock {
    FromValidator(SequencerBlockData),
    FromOtherValidator(SequencerBlockSubset),
    #[default]
    Default,
}

impl TryInto<SequencerBlockData> for HeadBlock {
    type Error = ();

    fn try_into(self) -> Result<SequencerBlockData, Self::Error> {
        use HeadBlock::*;
        match self {
            FromValidator(block) => Ok(block),
            FromOtherValidator(_) => Err(()),
            _ => unreachable!(),
        }
    }
}

impl HeadBlock {
    pub(crate) fn new_by_validator(block: SequencerBlockData) -> Self {
        Self::FromValidator(block)
    }

    pub(crate) fn new_by_other_validator(block: block::Response) -> Self {
        Self::FromOtherValidator(block.into())
    }

    pub(crate) fn block_hash(&self) -> Hash {
        use HeadBlock::*;
        match self {
            FromValidator(block) => block.block_hash(),
            FromOtherValidator(block) => block.block_hash,
            _ => unreachable!(),
        }
    }

    pub(crate) fn parent_block_hash(&self) -> Option<Hash> {
        use HeadBlock::*;
        match self {
            FromValidator(block) => block.parent_block_hash(),
            FromOtherValidator(block) => block.parent_block_hash,
            _ => unreachable!(),
        }
    }

    pub(crate) fn height(&self) -> u64 {
        use HeadBlock::*;
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
