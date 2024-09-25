use astria_core::{
    execution,
    generated::{
        self,
        bundle::v1alpha1::BaseBlock,
        sequencerblock::v1alpha1 as raw,
    },
    sequencerblock::v1alpha1::block::FilteredSequencerBlock,
    Protobuf,
};

// TODO: these should be created from the protos
#[derive(Debug, Clone)]
pub(crate) struct Optimistic {
    // TODO: actually convert this instead of just wrapping
    raw: raw::FilteredSequencerBlock,
}

impl Optimistic {
    pub(crate) fn from_raw(raw: raw::FilteredSequencerBlock) -> Self {
        Self {
            raw,
        }
    }

    pub(crate) fn into_raw(self) -> raw::FilteredSequencerBlock {
        self.raw
    }

    pub(crate) fn into_base_block(self) -> BaseBlock {
        unimplemented!()
    }

    fn into_executed_block(self, _executed_block: Executed) -> Executed {
        todo!()
    }

    fn into_block_commitment(self, _committed_block: Committed) -> Committed {
        todo!()
    }

    fn reorg(self) -> Self {
        todo!()
    }

    pub(crate) fn sequencer_block_hash(&self) -> [u8; 32] {
        FilteredSequencerBlock::try_from_raw(self.raw.clone())
            .unwrap()
            .block_hash()
            .clone()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Executed {
    rsp: execution::v1alpha2::Block,
}

impl Executed {
    pub(crate) fn from_raw(raw: generated::execution::v1alpha2::Block) -> Self {
        Self {
            rsp: execution::v1alpha2::Block::try_from_raw(raw).unwrap(),
        }
    }

    pub(crate) fn into_raw(self) -> execution::v1alpha2::Block {
        self.rsp
    }

    fn reorg(self) -> Optimistic {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Committed {
    raw: raw::SequencerBlockCommit,
}

impl Committed {
    pub(crate) fn from_raw(raw: raw::SequencerBlockCommit) -> Self {
        Self {
            raw,
        }
    }

    pub(crate) fn into_raw(self) -> raw::SequencerBlockCommit {
        self.raw
    }
}

// TODO: instead of state, should `CurrentBlock` just be:
pub(crate) struct CurrentBlock {
    optimistic: Optimistic,
    executed: Option<Executed>,
    committed: Option<Committed>,
}

impl CurrentBlock {
    pub(crate) fn apply_optimistic_block(self, _optimistic_block: Optimistic) -> Self {
        unimplemented!()
    }

    pub(crate) fn apply_executed_block(self, _executed_block: Executed) -> Self {
        unimplemented!()
    }

    pub(crate) fn apply_block_commitment(self, _block_commitment: Committed) -> Self {
        unimplemented!()
    }

    pub(crate) fn reorg(self, _opt: Optimistic) -> Result<(), CurrentBlockError> {
        unimplemented!()
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum CurrentBlockError {
    #[error("height out of order")]
    HeightOutOfOrder,
}
