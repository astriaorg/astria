use astria_core::{
    execution,
    generated::sequencerblock::v1alpha1::{
        SequencerBlockCommit,
        StreamOptimisticBlockResponse,
    },
};
use tokio::sync::watch;

// TODO: these should be created from the protos
#[derive(Debug, Clone)]
pub(crate) struct Optimistic {
    // TODO: actually convert this instead of just wrapping
    raw: StreamOptimisticBlockResponse,
}

impl Optimistic {
    pub(crate) fn from_raw(raw: StreamOptimisticBlockResponse) -> Self {
        Self {
            raw,
        }
    }

    pub(crate) fn into_raw(self) -> StreamOptimisticBlockResponse {
        self.raw
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
}

#[derive(Debug, Clone)]
pub(crate) struct Executed {
    raw: execution::v1alpha2::Block,
}

impl Executed {
    pub(crate) fn from_raw(raw: execution::v1alpha2::Block) -> Self {
        Self {
            raw,
        }
    }

    pub(crate) fn into_raw(self) -> execution::v1alpha2::Block {
        self.raw
    }

    fn into_exec_and_commit(self, _commit: Committed) -> ExecutedAndCommitted {
        todo!()
    }

    fn reorg(self) -> Optimistic {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Committed {
    raw: SequencerBlockCommit,
}

impl Committed {
    pub(crate) fn from_raw(raw: SequencerBlockCommit) -> Self {
        Self {
            raw,
        }
    }

    pub(crate) fn into_raw(self) -> SequencerBlockCommit {
        self.raw
    }

    fn into_exec_and_commit(self, _executed_block: Executed) -> ExecutedAndCommitted {
        todo!()
    }

    fn new_block(self) -> Optimistic {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExecutedAndCommitted(Executed, Committed);

impl ExecutedAndCommitted {
    fn new_block(self) -> Optimistic {
        todo!()
    }
}

#[derive(Debug, Clone)]
enum State {
    OptimisticBlock(Optimistic),
    ExecutedBlock(Executed),
    BlockCommitment(Committed),
    ExecutedAndCommitted(ExecutedAndCommitted),
}

impl State {
    fn handle_reorg(self, _new_block: Optimistic) -> Self {
        match self {
            State::OptimisticBlock(_) => todo!(),
            State::ExecutedBlock(_) => todo!(),
            State::BlockCommitment(_) => todo!(),
            State::ExecutedAndCommitted(_) => todo!(),
        }
    }
}

// TODO: instead of state, should `CurrentBlock` just be:
pub(crate) struct CurrentBlock {
    optimistic: Option<Optimistic>,
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

    // TODO: add getter funcs for current state or make fields pub(crate)
}
