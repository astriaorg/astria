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
    ContextCompat,
    OptionExt,
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
            .execute_optimistic_block_stream(self.rollup_id)
            .await
            .wrap_err("failed to stream executed blocks")?;

        // let bundle_stream = BundleServiceClient::new(bundle_service_grpc_url)
        //     .wrap_err("failed to initialize bundle service grpc client")?;

        // maybe just make this a fused future `auction_fut`
        let mut auction_futs: JoinMap<auction::Id, eyre::Result<()>> = JoinMap::new();
        let mut auction_handles: HashMap<auction::Id, auction::Handle> = HashMap::new();

        let mut curr_block: Option<CurrentBlock> = None;

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

                    // 1. block streams to update current block
                    //      1. optimistic block stream
                    optimistic_block = optimistic_stream_client.next() => {
                        // TODO: move this into stream domain type
                        let rsp = match optimistic_block {
                            Some(res) => res.wrap_err("received gRPC error")?,
                            None => break Err(eyre!("optimistic block stream closed by server")),
                        };
                        let opt = block::Optimistic::try_from_raw(rsp.block.unwrap()).wrap_err("failed to parse incoming optimistic block")?;

                        // TODO: should i have an if/let statement that logs if its none, instead?
                        // - this will probably only change once - might want to move that into startup instead of having the option<currentblock>
                        // - if the stream.next() returns the opt after sending it to the opt exec api (see note below), we don't need to clone here
                        let next_block = CurrentBlock::opt(opt.clone());

                        // 1. reorg by shutting down current auction fut, dumping the oneshot (this is the
                        //    state transition)
                        if let Some(hash) = curr_block.as_ref().map(|block| block.sequencer_block_hash()) {
                            let auction_id = auction::Id::from_sequencer_block_hash(hash);
                            let handle = auction_handles.get_mut(&auction_id).ok_or_eyre("unable to get handle for current auction")?;
                            handle.reorg()?;
                        };

                        // 2. start new auction fut in the joinmap with the sequencer block hash as key
                        // TODO:
                        // 1. make new auction and save the handle
                        // 2. add the handle.run() to the joinmap?

                        // 3. send new opt to the optimistic execution stream
                        // TODO: move this into stream domain type
                        // - this should just be part of the stream.next future probably, then we dont need the mpsc channel
                        let _ = opts_to_exec_tx.send_timeout(opt, Duration::from_millis(100));

                        // TODO: log the dumpingh of curr block?
                        curr_block = Some(next_block);
                    },

                    //      2. executed block stream
                    block_commitment = commit_stream_client.next() => {
                        // 1. update curr block state
                        //      - react if invalid state transition? maybe by dropping the block and its auction.
                        //          - altho this shouldnt happen because the block has to have an opt because of how cometbft and abci work,
                        //            assuming that the server implementation is correct.
                        //          - this is because cometbft should only be committing blocks/proposals that it has already processed,
                        //            since `Commit` only happens after voting in CometBFT. A `Commit` should only happen once per block
                        //          - if the server is committing blocks that it haven't been received on the optimistic stream, then an opt
                        //            message was either dropped or delayed for too long. This shouldn't happen since we're using UDS?
                        let raw = block_commitment.ok_or_eyre("stream failed")?.wrap_err("failed to receive block commitment")?.commitment.unwrap();
                        let commit = block::Committed::try_from_raw(raw).wrap_err("failed to parse incoming block commitment")?;


                        let next_block = curr_block
                            .map(|block| block.commit(commit).wrap_err("state transition failure"))
                            .expect("should only receive block commitment after optimistic block")?;
                        curr_block = Some(next_block);

                        // 2. send commit signal to the auction so it will start the timer
                            if let Some(hash) = curr_block.as_ref().map(|block| block.sequencer_block_hash()) {
                            let auction_id = auction::Id::from_sequencer_block_hash(hash);
                            let handle = auction_handles.get_mut(&auction_id).ok_or_eyre("unable to get handle for current auction")?;
                            handle.block_commitment()?;
                        }
                    },

                    //      3. block commitment stream
                    executed_block = executed_stream_client.next() => {
                        // 1. update curr block state (react if invalid state transition? maybe by dropping
                        //    the block and its auction)
                        let raw = executed_block
                            .wrap_err("executed block stream closed")?
                            .wrap_err("failed to receive executed block")?;
                        let exec = block::Executed::try_from_raw(raw).wrap_err("invalid executed block response")?;

                        let next_block = curr_block
                            .map(|block| block.exec(exec).wrap_err("state transition failure"))
                            .expect("should only receive executed blocks after an optimistic block has been received")?;
                        // TODO: probably can get rid of this
                        curr_block = Some(next_block);

                        // 2. send executed signal to the auction so it will start pulling bundles
                        if let Some(hash) = curr_block.as_ref().map(|block| block.sequencer_block_hash()) {
                            let auction_id = auction::Id::from_sequencer_block_hash(hash);
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
