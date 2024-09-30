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
    Committed,
    CurrentBlock,
    Executed,
    Optimistic,
};

pub(crate) struct Handle {
    block_rx: watch::Receiver<CurrentBlock>,
}

pub(crate) struct OptimisticExecutor {
    optimistic_blocks_rx: mpsc::Receiver<Optimistic>,
    executed_blocks_rx: mpsc::Receiver<Executed>,
    block_commitments_rx: mpsc::Receiver<Committed>,
    block: CurrentBlock,
}

impl OptimisticExecutor {
    pub(crate) async fn run(mut self) -> eyre::Result<()> {
        let Self {
            optimistic_blocks_rx: mut opt_rx,
            executed_blocks_rx: mut exec_rx,
            block_commitments_rx: mut commit_rx,
            mut block,
        } = self;

        // TODO: use grpc streams instead of channels

        // TODO: probably want to interact with the block state machine via the handle
        loop {
            let old_block = block.clone();
            let new_block = select! {
                opt = opt_rx.recv() => {
                    block.apply_optimistic_block(opt.unwrap())
                },
                exec = exec_rx.recv() => {
                    block.apply_executed_block(exec.unwrap())
                },
                commit = commit_rx.recv() => {
                    block.apply_block_commitment(commit.unwrap())
                },
            };

            info!(parent = ?old_block, block = ?new_block, "block state updated");
            block = new_block;
            // block_tx.send_modify(curr_state)
        }
    }
}
