use astria_sequencer_types::SequencerBlockData;
use tendermint::Hash;
use tendermint_rpc::endpoint::block;

#[derive(Clone, Default, Debug)]
pub(crate) enum HeadCandidate {
    ProposedByValidator(SequencerBlockData),
    CometBft(CometBftHeadCandidate),
    #[default]
    Default,
}

impl TryInto<SequencerBlockData> for HeadCandidate {
    type Error = ();

    fn try_into(self) -> Result<SequencerBlockData, Self::Error> {
        use HeadCandidate::*;
        match self {
            ProposedByValidator(block) => Ok(block),
            CometBft(_) => Err(()),
            _ => unreachable!(),
        }
    }
}

impl HeadCandidate {
    pub(crate) fn new_from_validator(block: SequencerBlockData) -> Self {
        Self::ProposedByValidator(block)
    }

    pub(crate) fn new_from_cometbft(block: block::Response) -> Self {
        Self::CometBft(block.into())
    }

    pub(crate) fn block_hash(&self) -> Hash {
        use HeadCandidate::*;
        match self {
            ProposedByValidator(block) => block.block_hash(),
            CometBft(block) => block.block_hash,
            _ => unreachable!(),
        }
    }

    pub(crate) fn parent_block_hash(&self) -> Option<Hash> {
        use HeadCandidate::*;
        match self {
            ProposedByValidator(block) => block.parent_block_hash(),
            CometBft(block) => block.parent_block_hash,
            _ => unreachable!(),
        }
    }

    pub(crate) fn height(&self) -> u64 {
        use HeadCandidate::*;
        match self {
            ProposedByValidator(block) => block.header().height.into(),
            CometBft(block) => block.height,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CometBftHeadCandidate {
    block_hash: Hash,
    parent_block_hash: Option<Hash>,
    height: u64,
}

impl From<block::Response> for CometBftHeadCandidate {
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
