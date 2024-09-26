use tokio::{
    select,
    sync::watch,
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
    // TODO: get rid of this, it should just be in the executor. the auction driver will recognize
    // the reorg by subscribing to state changes
    Reorg {
        old_block: ReorgedBlock,
        new_block: OptimisticBlock,
    },
}

pub(crate) struct Block {
    state: State,
}

impl Block {
    pub(crate) fn handle_opt(self, opt: OptimisticBlock) -> Self {
        let Self {
            state, ..
        } = self;

        let next_state = match state {
            State::OptimisticBlock(optimistic_block) => State::Reorg {
                old_block: ReorgedBlock::OptimisticBlock(optimistic_block),
                new_block: opt,
            },
            State::ExecutedBlock(executed_block) => State::Reorg {
                old_block: ReorgedBlock::ExecutedBlock(executed_block),
                new_block: opt,
            },
            State::BlockCommitment(_block_commitment) => State::OptimisticBlock(opt),
            State::Reorg {
                old_block: _old_block,
                new_block,
            } => State::OptimisticBlock(new_block),
        };

        Self {
            state: next_state,
        }
    }

    pub(crate) fn handle_commit(self, commit: BlockCommitment) -> Self {}

    pub(crate) async fn next_state(mut self) -> Block {
        // this should only do the state transitions and not the async stuff
        // channels should be driven from executor
        let Self {
            state,
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

    pub(crate) fn is_optimistic(&self) -> bool {
        match self.state {
            State::OptimisticBlock(_) => true,
            _ => false,
        }
    }

    pub(crate) fn is_executed(&self) -> bool {
        match self.state {
            State::ExecutedBlock(_) => true,
            _ => false,
        }
    }

    pub(crate) fn is_committed(&self) -> bool {
        match self.state {
            State::BlockCommitment(_) => true,
            _ => false,
        }
    }
}

pub(crate) struct Handle {
    rx: watch::Receiver<Block>,
}

impl Handle {
    // TODO: this will be called by the auction driver
    pub(crate) async fn next_state(&mut self) -> Block {
        self.rx.changed().await;
        todo!("return the new state after it is changed");
    }
}
