use tokio::{
    select,
    sync::{
        broadcast,
        mpsc,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct OptimisticBlock {}

#[derive(Debug, Clone)]
pub(crate) struct ExecutedBlock {}

#[derive(Debug, Clone)]
pub(crate) struct BlockCommitment {}

// TODO: should this be called UncleBlock?
#[derive(Debug, Clone)]
pub(crate) enum ReorgedBlock {
    OptimisticBlock(OptimisticBlock),
    ExecutedBlock(ExecutedBlock),
}

pub(crate) enum State {
    OptimisticBlock(OptimisticBlock),
    ExecutedBlock(ExecutedBlock),
    BlockCommitment(BlockCommitment),
    Reorg {
        old_block: ReorgedBlock,
        new_block: OptimisticBlock,
    },
}

pub(crate) struct Block {
    state: State,
    opt_rx: broadcast::Receiver<OptimisticBlock>,
    exec_rx: mpsc::Receiver<ExecutedBlock>,
    commit_rx: mpsc::Receiver<BlockCommitment>,
}

// TODO: this should be a future that is run by the auction driver with handles to channels pushed
// to by the optimistic executor
impl Block {
    pub(crate) async fn next_state(mut self) -> Block {
        let Self {
            state,
            mut opt_rx,
            mut exec_rx,
            mut commit_rx,
        } = self;

        let next = match state {
            State::OptimisticBlock(optimistic_block) => {
                select! {
                    exec = exec_rx.recv() => {
                        State::ExecutedBlock(exec.unwrap())
                    }
                    new_block = opt_rx.recv() => {
                        State::Reorg {
                           old_block: ReorgedBlock::OptimisticBlock(optimistic_block),
                           new_block: new_block.unwrap()
                        }
                    }
                }
            }
            State::ExecutedBlock(executed_block) => {
                select! {
                    commit = commit_rx.recv() => {
                        State::BlockCommitment(commit.unwrap())
                    }
                    new_block = opt_rx.recv() => {
                        State::Reorg {
                            old_block: ReorgedBlock::ExecutedBlock(executed_block),
                            new_block: new_block.unwrap()
                        }
                    }
                }
            }
            State::BlockCommitment(block_commitment) => {
                let new_block = opt_rx.recv().await.unwrap();
                State::OptimisticBlock(new_block)
            }
            State::Reorg {
                old_block,
                new_block,
            } => {
                // do something with old_block? maybe better to remove this arm and just handle this
                // in the reorg branches?
                State::OptimisticBlock(new_block)
            }
        };
        Block {
            state: next,
            opt_rx,
            exec_rx,
            commit_rx,
        }
    }

    pub(crate) fn state(&self) -> &State {
        &self.state
    }
}
