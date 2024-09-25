mod builder;
mod committed_block_stream;
mod executed_block_stream;
mod optimistic_block_stream;

use std::{
    collections::HashMap,
    time::Duration,
};

use astria_core::primitive::v1::RollupId;
use astria_eyre::eyre::{
    self,
    eyre,
    OptionExt as _,
};
use block::CurrentBlock;
pub(crate) use builder::Builder;
use optimistic_execution_client::OptimisticExecutionClient;
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
    optimistic_execution_grpc_endpoint: String,
    bundle_grpc_endpoint: String,
}

impl OptimisticExecutor {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let mut sequencer_client = SequencerGrpcClient::new(&self.sequencer_grpc_endpoint)
            .wrap_err("failed to initialize sequencer grpc client")?;

        // TODO: wrap these with saner domain types
        let mut optimistic_stream_client = sequencer_client
            .get_optimistic_block_stream(self.rollup_id)
            .await
            .wrap_err("failed to stream optimistic blocks")?;
        let mut commit_stream_client = sequencer_client
            .get_block_commitment_stream()
            .await
            .wrap_err("failed to stream block commitments")?;

        let mut optimistic_execution_client =
            OptimisticExecutionClient::new(&self.optimistic_execution_grpc_endpoint)
                .wrap_err("failed to initialize optimistic execution client")?;
        let (mut executed_stream_client, opts_to_exec_tx) = optimistic_execution_client
            .execute_optimistic_block_stream()
            .await
            .wrap_err("failed to stream executed blocks")?;

        // let bundle_stream = BundleServiceClient::new(bundle_service_grpc_url)
        //     .wrap_err("failed to initialize bundle service grpc client")?;

        // maybe just make this a fused future `auction_fut`
        let mut auction_futs: JoinMap<auction::Id, eyre::Result<()>> = JoinMap::new();
        let mut auction_handles: HashMap<auction::Id, auction::Handle> = HashMap::new();
        let mut curr_auction_id: Option<auction::Id> = None;

        let mut curr_block: Option<CurrentBlock> = None;

        let reason = {
            loop {
                select! {
                    biased;
                    () = self.shutdown_token.cancelled() => {
                        break Ok("received shutdown signal");
                    },

                    Some((id, res)) = auction_futs.join_next() => {
                        break flatten(res)
                            .wrap_err_with(|| "auction failed for block {id}")
                            .map(|_| "auction {id} failed");
                    },

                    // 1. block streams to update current block
                    //      1. optimistic block stream
                    optimistic_block = optimistic_stream_client.next() => {
                        let rsp = match optimistic_block {
                            Some(res) => res.wrap_err("received gRPC error")?,
                            None => break Err(eyre!("optimistic block stream closed by server")),
                        };
                        let opt = block::Optimistic::from_raw(rsp.block.unwrap());
                        let next_block = curr_block.map(|block| block.apply_optimistic_block(opt.clone()));

                        // 1. reorg by shutting down current auction fut, dumping the oneshot (this is the
                        //    state transition)
                        if let Some(auction_id) = curr_auction_id {
                            let handle = auction_handles.get_mut(&auction_id).ok_or_eyre("unable to get handle for current auction")?;
                            handle.reorg()?;
                        };

                        // 2. start new auction fut in the joinmap with the sequencer block hash as key
                        // TODO

                        // 3. send new opt to the optimistic execution stream
                        let _ = opts_to_exec_tx.send_timeout(opt, Duration::from_millis(100));

                        curr_block = next_block;
                    },

                    //      2. executed block stream
                    block_commitment = commit_stream_client.next() => {
                        // 1. update curr block state (react if invalid state transition? maybe by dropping
                        //    the block and its auction. altho this shouldnt happen because the block has
                        //    to have an opt)
                        let raw = block_commitment.unwrap().wrap_err("failed to receive block commitment")?.commitment.unwrap();
                        let commit = block::Committed::from_raw(raw);
                        let next_block = curr_block.map(|block| block.apply_block_commitment(commit));
                        curr_block = next_block;

                        // 2. send commit signal to the auction so it will start the timer
                        if let Some(auction_id) = curr_auction_id {
                            let handle = auction_handles.get_mut(&auction_id).ok_or_eyre("unable to get handle for current auction")?;
                            handle.block_commitment()?;
                        }
                    },

                    //      3. block commitment stream
                    executed_block = executed_stream_client.next() => {
                        // 1. update curr block state (react if invalid state transition? maybe by dropping
                        //    the block and its auction)
                        let raw = executed_block.unwrap().wrap_err("failed to receive executed block")?.block.unwrap();
                        let exec = block::Executed::from_raw(raw);
                        let next_block = curr_block.map(|block| block.apply_executed_block(exec));
                        curr_block = next_block;

                        // 2. send executed signal to the auction so it will start pulling bundles
                        if let Some(auction_id) = curr_auction_id {
                            let handle = auction_handles.get_mut(&auction_id).ok_or_eyre("unable to get handle for current auction")?;
                            handle.executed_block()?;
                        }
                    }


                    // 2. forward bundles from bundle stream into the correct auction fut
                    //      - bundles will build up in the channel into the auction until the executed signal is
                    //        sent to the auction fut. so if backpressure builds up here, i.e. bids arrive way
                    //        before execution, we can decide how to react here.
                    //        for example, we can drop all the bundles that are stuck in the channel and log a warning,
                    //        or we can kill the auction for that given block

                    // 3. execute the execute_fut if not terminated?
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

    async fn shutdown(self) {
        self.shutdown_token.cancel();
    }
}
