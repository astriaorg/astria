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
    sequencer_block: StreamOptimisticBlockResponse,
}

impl Optimistic {
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
    executed_block: execution::v1alpha2::Block,
}

impl Executed {
    fn into_exec_and_commit(self) -> Committed {
        todo!()
    }

    fn reorg(self) -> Optimistic {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Committed {
    commit: SequencerBlockCommit,
}

impl Committed {
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

#[derive(Debug, Clone)]
pub(crate) struct CurrentBlock {
    inner: State,
}

impl CurrentBlock {
    pub(crate) fn apply_optimistic_block(self, optimistic_block: Optimistic) -> Self {
        // check for reorg or new block to get the starting state
        let starting_state = self.inner.handle_reorg(optimistic_block);

        let new_state = match starting_state {
            State::OptimisticBlock(optimistic) => todo!(),
            State::ExecutedBlock(executed) => todo!(),
            State::BlockCommitment(committed) => todo!(),
            State::ExecutedAndCommitted(executed_and_committed) => todo!(),
        };

        Self {
            inner: new_state,
        }
    }

    pub(crate) fn apply_executed_block(self, executed_block: Executed) -> Self {
        let new_state = match self.inner {
            State::OptimisticBlock(optimistic) => {
                State::ExecutedBlock(optimistic.into_executed_block(executed_block))
            }
            State::BlockCommitment(committed) => {
                State::ExecutedAndCommitted(committed.into_exec_and_commit(executed_block))
            }
            State::ExecutedBlock(executed) => panic!("double executed block"),
            State::ExecutedAndCommitted(executed_and_committed) => panic!("double executed block"),
        };

        Self {
            inner: new_state,
        }
    }

    pub(crate) fn apply_block_commitment(self, block_commitment: Committed) -> Self {
        let new_state = match self.inner {
            State::OptimisticBlock(optimistic) => todo!(),
            State::ExecutedBlock(executed) => todo!(),
            State::BlockCommitment(committed) => todo!(),
            State::ExecutedAndCommitted(executed_and_committed) => todo!(),
        };

        Self {
            inner: new_state,
        }
    }
}

pub(crate) struct Handle {
    rx: watch::Receiver<CurrentBlock>,
}

impl Handle {
    // TODO: this will be called by the auction driver
    pub(crate) async fn get_block(&mut self) -> CurrentBlock {
        self.rx.changed().await;
        todo!("return the new state after it is changed");
    }
}
