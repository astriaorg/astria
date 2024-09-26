use astria_core::{
    execution,
    generated::sequencerblock::v1alpha1::{
        SequencerBlockCommit,
        StreamOptimisticBlockResponse,
    },
};
use tokio::sync::watch;
use tracing::info;

// TODO: these should be created from the protos
#[derive(Debug, Clone)]
pub(crate) struct OptimisticBlock {
    sequencer_block: StreamOptimisticBlockResponse,
}

#[derive(Debug, Clone)]
pub(crate) struct ExecutedBlock {
    executed_block: execution::v1alpha2::Block,
}

#[derive(Debug, Clone)]
pub(crate) struct BlockCommitment {
    commit: SequencerBlockCommit,
}

// TODO: should this be called UncleBlock?
#[derive(Debug, Clone)]
pub(crate) enum ReorgedBlock {
    OptimisticBlock(OptimisticBlock),
    ExecutedBlock(ExecutedBlock),
}

#[derive(Debug, Clone)]
pub(crate) enum State {
    OptimisticBlock(OptimisticBlock),
    ExecutedBlock(ExecutedBlock),
    BlockCommitment(BlockCommitment),
    ExecutedAndCommitted(ExecutedBlock, BlockCommitment),
}

impl State {
    pub(crate) fn apply_state(self, next_state: State) -> State {
        match next_state {
            State::OptimisticBlock(optimistic_block) => self.handle_opt(optimistic_block),
            State::ExecutedBlock(executed_block) => self.handle_exec(executed_block),
            State::BlockCommitment(block_commitment) => self.handle_commit(block_commitment),
            State::ExecutedAndCommitted(executed_block, block_commitment) => {
                self.handle_exec_and_commit(executed_block, block_commitment)
            }
        }
    }

    pub(crate) fn handle_opt(self, opt: OptimisticBlock) -> State {
        match self {
            State::OptimisticBlock(_) => State::OptimisticBlock(opt),
            State::ExecutedBlock(executed_block) => State::OptimisticBlock(opt),
            State::BlockCommitment(_) => todo!("missed exec from previous round?"),
            State::ExecutedAndCommitted(..) => {
                todo!("handle reorg??");
                State::OptimisticBlock(opt)
            }
        }
    }

    pub(crate) fn handle_exec(self, exec: ExecutedBlock) -> State {
        match self {
            State::OptimisticBlock(optimistic_block) => todo!(),
            State::ExecutedBlock(executed_block) => todo!(),
            State::BlockCommitment(block_commitment) => todo!(),
            State::ExecutedAndCommitted(executed_block, block_commitment) => todo!(),
        }
    }

    pub(crate) fn handle_commit(self, commit: BlockCommitment) -> State {
        match self {
            State::OptimisticBlock(optimistic_block) => todo!(),
            State::ExecutedBlock(executed_block) => todo!(),
            State::BlockCommitment(block_commitment) => todo!(),
            State::ExecutedAndCommitted(executed_block, block_commitment) => todo!(),
        }
    }

    pub(crate) fn handle_exec_and_commit(
        self,
        exec: ExecutedBlock,
        commit: BlockCommitment,
    ) -> State {
        match self {
            State::OptimisticBlock(_opt) => {
                // TODO: make sure that exec and commit
                State::ExecutedAndCommitted(exec, commit)
            }
            State::ExecutedBlock(_) => todo!("missed opt"),
            State::BlockCommitment(_) => todo!("missed opt and exec"),
            State::ExecutedAndCommitted(..) => todo!("missed opt and exec and commit"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CurrentBlock {
    Block(State),
    Reorg { old_state: State, new_state: State },
}

impl CurrentBlock {
    pub(crate) fn apply_state(self, next_state: State) -> Self {
        let curr_state = match self {
            CurrentBlock::Block(state) => state,
            CurrentBlock::Block(old_state) => old_state,
            CurrentBlock::Reorg {
                old_state,
                new_state,
            } => {
                info!(old = ?old_state, new = ?new_state, "reorging");
                new_state
            }
        };

        let next_state = curr_state.handle_opt(opt);

        Self::Block(next_state)
    }

    // pub(crate) async fn next_state(mut self) -> Block {
    //     // this should only do the state transitions and not the async stuff
    //     // channels should be driven from executor
    //     let Self {
    //         state,
    //     } = self;

    //     let next = match state {
    //         State::OptimisticBlock(optimistic_block) => {
    //             select! {
    //                 exec = exec_rx.recv() => {
    //                     State::ExecutedBlock(exec.unwrap())
    //                 }
    //                 new_block = opt_rx.recv() => {
    //                     State::Reorg {
    //                        old_block: ReorgedBlock::OptimisticBlock(optimistic_block),
    //                        new_block: new_block.unwrap()
    //                     }
    //                 }
    //             }
    //         }
    //         State::ExecutedBlock(executed_block) => {
    //             select! {
    //                 commit = commit_rx.recv() => {
    //                     State::BlockCommitment(commit.unwrap())
    //                 }
    //                 new_block = opt_rx.recv() => {
    //                     State::Reorg {
    //                         old_block: ReorgedBlock::ExecutedBlock(executed_block),
    //                         new_block: new_block.unwrap()
    //                     }
    //                 }
    //             }
    //         }
    //         State::BlockCommitment(block_commitment) => {
    //             let new_block = opt_rx.recv().await.unwrap();
    //             State::OptimisticBlock(new_block)
    //         }
    //         State::Reorg {
    //             old_block,
    //             new_block,
    //         } => {
    //             // do something with old_block? maybe better to remove this arm and just handle
    // this             // in the reorg branches?
    //             State::OptimisticBlock(new_block)
    //         }
    //     };
    //     Block {
    //         state: next,
    //     }
    // }

    pub(crate) fn is_optimistic(&self) -> bool {
        todo!("match internally");
    }

    pub(crate) fn is_executed(&self) -> bool {
        todo!("match internally");
    }

    pub(crate) fn is_committed(&self) -> bool {
        todo!("match internally");
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
