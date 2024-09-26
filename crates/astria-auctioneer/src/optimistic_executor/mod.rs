mod builder;
mod committed_block_stream;
mod executed_block_stream;
mod optimistic_block_stream;

use astria_eyre::eyre;
pub(crate) use builder::Builder;
use tokio::{
    select,
    sync::{
        mpsc,
        watch,
    },
};
use tracing::info;

use crate::block::{
    self,
    BlockCommitment,
    CurrentBlock,
    ExecutedBlock,
    OptimisticBlock,
};

pub(crate) struct Handle {
    block_rx: watch::Receiver<CurrentBlock>,
}

pub(crate) struct OptimisticExecutor {
    optimistic_blocks_rx: mpsc::Receiver<OptimisticBlock>,
    executed_blocks_rx: mpsc::Receiver<ExecutedBlock>,
    block_commitments_rx: mpsc::Receiver<BlockCommitment>,
    block: CurrentBlock,
}

impl OptimisticExecutor {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let Self {
            optimistic_blocks_rx: mut opt_rx,
            executed_blocks_rx: mut exec_rx,
            block_commitments_rx: mut commit_rx,
            block,
        } = self;

        // TODO: probably want to interact with the block state machine via the handle
        let mut curr_block = block.clone();
        loop {
            let old_block = curr_block.clone();
            let new_block = select! {
                opt = opt_rx.recv() => {
                    curr_block.apply_state(block::State::OptimisticBlock(opt.unwrap()))
                },
                exec = exec_rx.recv() => {
                    curr_block.apply_state(block::State::ExecutedBlock(exec.unwrap()))
                },
                commit = commit_rx.recv() => {
                    curr_block.apply_state(block::State::BlockCommitment(commit.unwrap()))
                },
            };

            info!(parent = ?old_block, block = ?new_block, "block state updated");
            curr_block = new_block;
            // block_tx.send_modify(curr_state)
        }
    }
}
