mod builder;

use std::time::Duration;

use astria_core::primitive::v1::{
    asset,
    RollupId,
};
use astria_eyre::eyre::{
    self,
    OptionExt,
    WrapErr as _,
};
pub(crate) use builder::Builder;
use telemetry::display::base64;
use tokio::select;
use tokio_stream::StreamExt as _;
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    instrument,
};

use crate::{
    auction,
    block::{
        self,
        block_commitment_stream::BlockCommitmentStream,
        executed_stream::ExecutedBlockStream,
        optimistic_stream::OptimisticBlockStream,
    },
    bundle::{
        Bundle,
        BundleStream,
    },
    optimistic_block_client::OptimisticBlockClient,
};

pub(crate) struct Startup {
    #[allow(dead_code)]
    metrics: &'static crate::Metrics,
    shutdown_token: CancellationToken,
    sequencer_grpc_endpoint: String,
    rollup_id: RollupId,
    rollup_grpc_endpoint: String,
    auctions: auction::Manager,
}

impl Startup {
    pub(crate) async fn startup(self) -> eyre::Result<Running> {
        let Self {
            metrics,
            shutdown_token,
            sequencer_grpc_endpoint,
            rollup_id,
            rollup_grpc_endpoint,
            auctions,
        } = self;

        let sequencer_client = OptimisticBlockClient::new(&sequencer_grpc_endpoint)
            .wrap_err("failed to initialize sequencer grpc client")?;
        // TODO: have a connect streams helper?
        let mut optimistic_blocks =
            OptimisticBlockStream::connect(rollup_id, sequencer_client.clone())
                .await
                .wrap_err("failed to initialize optimsitic block stream")?;

        let block_commitments = BlockCommitmentStream::connect(sequencer_client)
            .await
            .wrap_err("failed to initialize block commitment stream")?;

        let (blocks_to_execute_handle, executed_blocks) =
            ExecutedBlockStream::connect(rollup_id, rollup_grpc_endpoint.clone())
                .await
                .wrap_err("failed to initialize executed block stream")?;

        let bundle_stream = BundleStream::connect(rollup_grpc_endpoint)
            .await
            .wrap_err("failed to initialize bundle stream")?;
        // let bundle_stream = BundleServiceClient::new(bundle_service_grpc_url)
        //     .wrap_err("failed to initialize bundle service grpc client")?;

        let optimistic_block = optimistic_blocks
            .next()
            .await
            .ok_or_eyre("optimistic stream closed during startup?")?
            .wrap_err("failed to get optimistic block during startup")?;
        let current_block = block::Current::with_optimistic(optimistic_block);

        Ok(Running {
            metrics,
            shutdown_token,
            optimistic_blocks,
            block_commitments,
            executed_blocks,
            blocks_to_execute_handle,
            bundle_stream,
            auctions,
            current_block,
        })
    }
}

pub(crate) struct Running {
    metrics: &'static crate::Metrics,
    shutdown_token: CancellationToken,
    optimistic_blocks: OptimisticBlockStream,
    block_commitments: BlockCommitmentStream,
    executed_blocks: ExecutedBlockStream,
    blocks_to_execute_handle: block::executed_stream::Handle,
    bundle_stream: BundleStream,
    auctions: auction::Manager,
    current_block: block::Current,
}

impl Running {
    pub(crate) async fn run(mut self) -> eyre::Result<()> {
        let reason: eyre::Result<&str> = {
            loop {
                select! {
                    biased;
                    () = self.shutdown_token.cancelled() => {
                        break Ok("received shutdown signal");
                    },

                    Some((id, res)) = self.auctions.join_next() => {
                        // TODO: why doesnt this use `id`
                        res.wrap_err_with(|| "auction failed for block {id}").map(|_| "auction {id} failed")?;
                    },

                    Some(res) = self.optimistic_blocks.next() => {
                        let optimistic_block = res.wrap_err("failed to get optimistic block")?;

                        self.optimistic_block_handler(optimistic_block).wrap_err("failed to handle optimistic block")?;
                    },

                    Some(res) = self.block_commitments.next() => {
                        let block_commitment = res.wrap_err("failed to get block commitment")?;

                        self.block_commitment_handler(block_commitment).wrap_err("failed to handle block commitment")?;

                    },

                    Some(res) = self.executed_blocks.next() => {
                        let executed_block = res.wrap_err("failed to get executed block")?;

                        self.executed_block_handler(executed_block).wrap_err("failed to handle executed block")?;
                    }

                    Some(res) = self.bundle_stream.next() => {
                        let bundle = res.wrap_err("failed to get bundle")?;

                        self.bundle_handler(bundle).wrap_err("failed to handle bundle")?;
                    }
                }
            }
        };

        match reason {
            Ok(msg) => info!(%msg, "shutting down"),
            Err(err) => error!(%err, "shutting down due to error"),
        };

        self.shutdown().await;
        Ok(())
    }

    #[instrument(skip(self), fields(auction.old_id = %base64(self.current_block.sequencer_block_hash())))]
    fn optimistic_block_handler(
        &mut self,
        optimistic_block: block::Optimistic,
    ) -> eyre::Result<()> {
        let old_auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());
        self.auctions
            .abort_auction(old_auction_id)
            .wrap_err("failed to abort auction")?;

        info!(
            // TODO: is this how we display block hashes?
            optimistic_block.sequencer_block_hash = %base64(optimistic_block.sequencer_block_hash()),
            "received optimistic block, aborting old auction and starting new auction"
        );

        self.current_block = block::Current::with_optimistic(optimistic_block.clone());
        let new_auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());
        self.auctions.new_auction(new_auction_id);

        // create and run the auction fut and save the its handles
        // forward the optimistic block to the rollup's optimistic execution server
        // TODO: don't want to exit on this, just complain with a log and skip the block or smth
        self.blocks_to_execute_handle
            .try_send_block_to_execute(optimistic_block)
            .wrap_err("failed to send optimistic block for execution")?;

        Ok(())
    }

    #[instrument(skip(self), fields(auction.id = %base64(self.current_block.sequencer_block_hash())))]
    fn block_commitment_handler(
        &mut self,
        block_commitment: block::Commitment,
    ) -> eyre::Result<()> {
        // TODO: handle this with a log instead of exiting
        self.current_block
            .commitment(block_commitment)
            .wrap_err("failed to set block commitment")?;

        let auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());
        self.auctions
            .start_timer(auction_id)
            .wrap_err("failed to start timer")?;

        Ok(())
    }

    #[instrument(skip(self), fields(auction.id = %base64(self.current_block.sequencer_block_hash())))]
    fn executed_block_handler(&mut self, executed_block: block::Executed) -> eyre::Result<()> {
        // TODO: handle this with a log instead of exiting
        self.current_block
            .execute(executed_block)
            .wrap_err("failed to set block to executed")?;

        let auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());
        self.auctions
            .start_processing_bids(auction_id)
            .wrap_err("failed to start processing bids")?;

        Ok(())
    }

    #[instrument(skip(self), fields(auction.id = %base64(self.current_block.sequencer_block_hash())))]
    fn bundle_handler(&mut self, bundle: Bundle) -> eyre::Result<()> {
        let auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());
        self.auctions
            .try_send_bundle(auction_id, bundle)
            .wrap_err("failed to submit bundle to auction")?;

        Ok(())
    }

    async fn shutdown(self) {
        self.shutdown_token.cancel();
    }
}
