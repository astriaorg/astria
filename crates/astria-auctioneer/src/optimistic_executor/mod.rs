mod builder;
mod committed_block_stream;
mod executed_block_stream;
mod optimistic_block_stream;

use std::sync::Arc;

pub(crate) use builder::Builder;
use tokio::sync::{
    mpsc,
    watch,
};

use crate::block::{
    Block,
    BlockCommitment,
    ExecutedBlock,
    OptimisticBlock,
};

pub(crate) struct Handle {
    block_rx: watch::Receiver<Block>,
}

pub(crate) struct OptimisticExecutor {
    opt_rx: mpsc::Receiver<OptimisticBlock>,
    exec_rx: mpsc::Receiver<ExecutedBlock>,
    commit_rx: mpsc::Receiver<BlockCommitment>,
    block: Arc<Block>,
}

impl OptimisticExecutor {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let Self {
            mut opt_rx,
            mut exec_rx,
            mut commit_rx,
            block,
        } = self;

        loop {
            let new_block = tokio::select! {
                opt = opt_rx.recv() => {

                },
                exec = exec_rx.recv() => {
                },
                commit = commit_rx.recv() => {
                },
            };
        }
    }
}
