mod builder;
mod committed_block_stream;
mod executed_block_stream;
mod optimistic_block_stream;

use astria_core::primitive::v1::RollupId;
use astria_eyre::eyre;
use block::CurrentBlock;
pub(crate) use builder::Builder;
use futures::future::Fuse;
use sequencer_client::SequencerGrpcClient;
use tokio::select;
use tokio_stream::StreamExt as _;
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tracing::{
    error,
    info,
    instrument::Instrumented,
};

mod optimistic_execution_client;
mod sequencer_client;
use astria_eyre::eyre::WrapErr as _;

use crate::{
    auction,
    block,
    flatten,
};

pub(crate) struct OptimisticExecutor {
    #[allow(dead_code)]
    metrics: &'static crate::Metrics,
    shutdown_token: CancellationToken,
    sequencer_grpc_endpoint: String,
    rollup_id: RollupId,
}

impl OptimisticExecutor {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let mut sequencer_client = SequencerGrpcClient::new(&self.sequencer_grpc_endpoint)
            .wrap_err("failed to initialize sequencer grpc client")?;

        let mut optimistic_stream_client = sequencer_client
            .optimistic_block_stream(self.rollup_id)
            .await
            .wrap_err("failed to stream optimistic blocks")?;
        let mut commit_stream_client = sequencer_client
            .block_commitment_stream()
            .await
            .wrap_err("failed to stream block commitments")?;

        // let executed_stream_client = sequencer_client
        //     .executed_block_stream(rollup_id)
        //     .await
        //     .wrap_err("failed to stream executed blocks")?;

        // let bundle_stream = BundleServiceClient::new(bundle_service_grpc_url)
        //     .wrap_err("failed to initialize bundle service grpc client")?;

        // loop over:
        // 1. block streams to update current block
        //      1. optimistic block stream
        //          1. reorg by shutting down current auction fut, dumping the oneshot  (this is the
        //             state transition)
        //          2. start new auction fut in the joinmap with the sequencer block hash as key
        //          3. send new execute_fut to the optimistic execution stream
        //      2. executed block stream
        //          1. update curr block state (react if invalid state transition? maybe by dropping
        //             the block and its auction)
        //          2. send executed signal to the auction so it will start pulling bundles
        //      3. block commitment stream
        //          1. update curr block state (react if invalid state transition? maybe by dropping
        //             the block and its auction. altho this shouldnt happen because the block has
        //             to have an opt)
        //          2. send commit signal to the auction so it will start the timer
        // 2. forward bundles from bundle stream into the correct auction fut
        //      - bundles will build up in the channel into the auction until the executed signal is
        //        sent to the auction fut. so if backpressure builds up here, i.e. bids arrive way
        //        before execution, we can decide how to react here. for example, we can drop all
        //        the bundles that are stuck in the channel and log a warning, or we can kill the
        //        auction for that given block
        // 3. execute the execute_fut if not terminated?
        // 4. handle auction_map.join_next() somehow
        // 5. cancellation token or something
        //

        // maybe just make this a fused future `auction_fut`
        let mut execution_fut: Fuse<Instrumented<ExecutionFut>> = Fuse::terminated();
        let mut auction_futs: JoinMap<auction::Id, eyre::Result<auction::Winner>> = JoinMap::new();
        let mut curr_block: Option<CurrentBlock> = None;

        let reason = {
            loop {
                select! {
                    biased;
                    () = self.shutdown_token.cancelled() => {
                        break Ok("received shutdown signal");
                    },

                    Some((id, res)) = auction_futs.join_next() => {
                        let id = id.sequencer_block_hash;
                        break flatten(res)
                            .wrap_err_with(|| "auction failed for block {id}")
                            .map(|_| "auction {id} failed");
                    },

                    optimistic_block = optimistic_stream_client.next() => {
                        let opt = block::Optimistic::from_raw(optimistic_block.unwrap().wrap_err("failed to receive optimistic block")?);
                        let next_block = curr_block.map(|block| block.apply_optimistic_block(opt));
                        curr_block = next_block;
                    },
                    block_commitment = commit_stream_client.next() => {
                        let commit = block::Committed::from_raw(block_commitment.unwrap().wrap_err("failed to receive block commitment")?);
                        let next_block = curr_block.map(|block| block.apply_block_commitment(commit));
                        curr_block = next_block;
                    },
                }
            }
        };

        match reason {
            Ok(msg) => info!(%msg, "shutting down"),
            Err(err) => error!(%err, "shutting down due to error"),
        };

        self.shutdown();
        Ok(())
    }

    async fn shutdown(mut self) {
        self.shutdown_token.cancel();
    }
}
