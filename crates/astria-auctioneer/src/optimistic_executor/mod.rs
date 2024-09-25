mod builder;

use std::{
    collections::HashMap,
    time::Duration,
};

use astria_core::primitive::v1::RollupId;
use astria_eyre::eyre::{
    self,
    OptionExt,
    WrapErr as _,
};
use block::CurrentBlock;
pub(crate) use builder::Builder;
use telemetry::display::base64;
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

use super::optimistic_execution_client::OptimisticExecutionClient;
use crate::{
    auction,
    block::{
        self,
        commitment_stream::BlockCommitmentStream,
        executed_stream::ExecutedBlockStream,
        optimistic_stream::OptimisticBlockStream,
    },
    flatten,
};

pub(crate) struct OptimisticExecutor {
    #[allow(dead_code)]
    metrics: &'static crate::Metrics,
    shutdown_token: CancellationToken,
    sequencer_grpc_endpoint: String,
    sequencer_abci_endpoint: String,
    rollup_id: RollupId,
    rollup_grpc_endpoint: String,
    bundle_grpc_endpoint: String,
    latency_margin: Duration,
}

impl OptimisticExecutor {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let mut optimistic_stream =
            OptimisticBlockStream::new(self.rollup_id, self.sequencer_grpc_endpoint.clone())
                .await
                .wrap_err("failed to initialize optimsitic block stream")?;

        let mut block_commitment_stream =
            BlockCommitmentStream::new(self.sequencer_grpc_endpoint.clone())
                .await
                .wrap_err("failed to initialize block commitment stream")?;

        let (mut exec_stream_handle, mut executed_block_stream) =
            ExecutedBlockStream::new(self.rollup_id, self.rollup_grpc_endpoint.clone())
                .await
                .wrap_err("failed to initialize executed block stream")?;

        // let bundle_stream = BundleServiceClient::new(bundle_service_grpc_url)
        //     .wrap_err("failed to initialize bundle service grpc client")?;

        // maybe just make this a fused future `auction_fut`
        let mut auction_futs: JoinMap<auction::Id, eyre::Result<()>> = JoinMap::new();
        let mut auction_oe_handles: HashMap<auction::Id, auction::OptimisticExecutionHandle> =
            HashMap::new();
        let mut auction_bundle_handles: HashMap<auction::Id, auction::BundlesHandle> =
            HashMap::new();

        let optimistic_block = optimistic_stream
            .next()
            .await
            .ok_or_eyre("optimistic stream closed during startup?")?
            .wrap_err("failed to get optimistic block during startup")?;
        let mut curr_block = Some(CurrentBlock::opt(optimistic_block));

        let reason = {
            loop {
                select! {
                    biased;
                    () = self.shutdown_token.cancelled() => {
                        break Ok("received shutdown signal");
                    },

                    Some((_id, res)) = auction_futs.join_next() => {
                        // TODO: fix this
                        break flatten(res)
                            .wrap_err_with(|| "auction failed for block {id}")
                            .map(|_| "auction {id} failed");
                    },

                    Some(res) = optimistic_stream.next() => {
                        // move into self.process_optimistic_block() or somethingg
                        let optimistic_block = res.wrap_err("failed to get optimistic block")?;

                        // reorg by shutting down current auction fut, dumping the oneshot (this is the
                        // state transition) and replace the current block with the optimistic block
                        let auction_id = auction::Id::from_sequencer_block_hash(optimistic_block.sequencer_block_hash());
                        if let Some(curr_block) = curr_block {
                            let handle = auction_oe_handles.get_mut(&auction_id).ok_or_eyre("unable to get handle for current auction")?;
                            handle.reorg()?;

                            info!(
                                // TODO: is this how we display block hashes?
                                optimistic_block.sequencer_block_hash = %base64(optimistic_block.sequencer_block_hash()),
                                current_block.sequencer_block_hash = %base64(curr_block.sequencer_block_hash()),
                               "replacing current block with optimistic block");

                        };
                        curr_block = Some(CurrentBlock::opt(optimistic_block.clone()));

                        // create and run the auction fut and save the its handles
                        let (auction_driver, optimistic_execution_handle, bundles_handle) = auction::Builder {
                            metrics: self.metrics,
                            shutdown_token: self.shutdown_token.clone(),
                            sequencer_grpc_endpoint: self.sequencer_grpc_endpoint.clone(),
                            sequencer_abci_endpoint: self.sequencer_abci_endpoint.clone(),
                            latency_margin: self.latency_margin,
                            auction_id,
                        }.build().wrap_err("failed to build auction")?;

                        // TODO: clean this up?
                        auction_futs.spawn(auction_id, auction_driver.run());
                        auction_oe_handles.insert(auction_id, optimistic_execution_handle);
                        auction_bundle_handles.insert(auction_id, bundles_handle);

                        // forward the optimistic block to the rollup's optimistic execution server
                        exec_stream_handle
                            .try_send_block_to_execute(optimistic_block)
                            .wrap_err("failed to send optimistic block for execution")?;
                    },

                    Some(res) = block_commitment_stream.next() => {
                        let block_commitment = res.wrap_err("failed to get block commitment")?;

                        if let Some(block) = curr_block {
                            curr_block = Some(block.commit(block_commitment).wrap_err("state transition failure")?);
                        }

                        // 2. send commit signal to the auction so it will start the timer
                        if let Some(hash) = curr_block.as_ref().map(|block| block.sequencer_block_hash()) {
                            let auction_id = auction::Id::from_sequencer_block_hash(hash);
                            let handle = auction_oe_handles.get_mut(&auction_id).ok_or_eyre("unable to get handle for current auction")?;
                            handle.block_commitment()?;
                        }
                    },

                    //      3. block commitment stream
                    Some(res) = executed_block_stream.next() => {
                        // 1. update curr block state (react if invalid state transition? maybe by dropping
                        //    the block and its auction)
                        let executed_block = res.wrap_err("failed to get executed block")?;

                        let next_block = curr_block
                            .map(|block| block.exec(executed_block).wrap_err("state transition failure"))
                            .expect("should only receive executed blocks after an optimistic block has been received")?;
                        // TODO: probably can get rid of this
                        curr_block = Some(next_block);

                        // 2. send executed signal to the auction so it will start pulling bundles
                        if let Some(hash) = curr_block.as_ref().map(|block| block.sequencer_block_hash()) {
                            let auction_id = auction::Id::from_sequencer_block_hash(hash);
                            let handle = auction_oe_handles.get_mut(&auction_id).ok_or_eyre("unable to get handle for current auction")?;
                            handle.executed_block()?;
                        }
                    }


                    // 2. forward bundles from bundle stream into the correct auction fut
                    //      - bundles will build up in the channel into the auction until the executed signal is
                    //        sent to the auction fut. so if backpressure builds up here, i.e. bids arrive way
                    //        before execution, we can decide how to react here.
                    //        for example, we can drop all the bundles that are stuck in the channel and log a warning,
                    //        or we can kill the auction for that given block
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
