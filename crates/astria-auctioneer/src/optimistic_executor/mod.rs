mod builder;
mod committed_block_stream;
mod executed_block_stream;
mod optimistic_block_stream;

use astria_core::primitive::v1::RollupId;
use astria_eyre::eyre;
pub(crate) use builder::Builder;
use optimistic_block_stream::OptimisticBlockStream;
use sequencer_client::SequencerGrpcClient;
use tokio::{
    select,
    sync::{
        mpsc,
        watch,
    },
};
use tokio_stream::StreamExt as _;
use tracing::info;

use crate::block::{
    Committed,
    CurrentBlock,
    Executed,
    Optimistic,
};

mod sequencer_client;
use astria_eyre::eyre::WrapErr as _;

pub(crate) struct Handle {
    block_rx: watch::Receiver<CurrentBlock>,
}

pub(crate) struct OptimisticExecutor {
    sequencer_grpc_url: String,
    rollup_id: RollupId,
    optimistic_blocks_rx: mpsc::Receiver<Optimistic>,
    executed_blocks_rx: mpsc::Receiver<Executed>,
    block_commitments_rx: mpsc::Receiver<Committed>,
    // watch::Sender<CurrentBlock>,
    block: CurrentBlock,
}

impl OptimisticExecutor {
    pub(crate) async fn run(mut self) -> eyre::Result<()> {
        let Self {
            sequencer_grpc_url,
            rollup_id,
            executed_blocks_rx: mut exec_rx,
            block_commitments_rx: mut commit_rx,
            mut block,
            ..
        } = self;

        // TODO: use grpc streams instead of channels
        let mut sequencer_client = SequencerGrpcClient::new(&sequencer_grpc_url)
            .wrap_err("failed to initialize sequencer grpc client")?;
        let stream_client = sequencer_client
            .optimistic_block_stream(rollup_id)
            .await
            .wrap_err("failed to stream optimistic blocks")?;
        let mut optimistic_block_stream = OptimisticBlockStream::new(stream_client);

        // TODO: probably want to interact with the block state machine via the handle
        loop {
            let old_block = block.clone();
            let new_block = select! {
                optimistic_block = optimistic_block_stream.next() => {
                    // TODO: stop doing this unwrap unwrap thing with the stream
                    // curr_block = block.borrow();
                    // let next = curr_block.apply_optimistic_block(optimistic_block.unwrap().unwrap())
                    // block.send(next);


                    // TODO: add execute_fut fused future that sends this to the optimistic execution stream
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
