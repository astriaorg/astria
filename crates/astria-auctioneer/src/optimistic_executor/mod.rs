//! - [ ] mention backpressure here

use astria_core::primitive::v1::RollupId;
use astria_eyre::eyre::{
    self,
    eyre,
    OptionExt,
    WrapErr as _,
};
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
        commitment_stream::BlockCommitmentStream,
        executed_stream::ExecutedBlockStream,
        optimistic_stream::OptimisticBlockStream,
    },
    bundle::{
        Bundle,
        BundleStream,
    },
    optimistic_block_client::OptimisticBlockClient,
};

mod builder;
pub(crate) use builder::Builder;

macro_rules! break_for_closed_stream {
    ($stream_res:expr, $msg:expr) => {
        match $stream_res {
            Some(val) => val,
            None => break Err(eyre!($msg)),
        }
    };
}

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
            // This is a long running loop. Errors are emitted inside the handlers.
            loop {
                select! {
                    biased;
                    () = self.shutdown_token.cancelled() => {
                        break Ok("received shutdown signal");
                    },

                    Some((id, res)) = self.auctions.join_next() => {
                        // TODO: this seems wrong?
                        res.wrap_err_with(|| "auction failed for block {id}").map(|_| "auction {id} failed")?;
                    },

                    res = self.optimistic_blocks.next() => {
                        let res = break_for_closed_stream!(res, "optimistic block stream closed");

                        let _ = self.handle_optimistic_block(res);
                    },

                    Some(res) = self.block_commitments.next() => {
                        let block_commitment = tri!(res.wrap_err("failed to get block commitment"));

                        let _ = self.handle_block_commitment(block_commitment);

                    },

                    Some(res) = self.executed_blocks.next() => {
                        let executed_block = res.wrap_err("failed to get executed block")?;

                        let _ = self.handle_executed_block(executed_block);
                    }

                    Some(res) = self.bundle_stream.next() => {
                        let bundle = res.wrap_err("failed to get bundle")?;

                        let _ = self.handle_incoming_bundle(bundle);
                    }
                }
            }
        };

        match reason {
            Ok(msg) => info!(%msg, "shutting down"),
            Err(err) => error!(%err, "shutting down due to error"),
        };

        Ok(())
    }

    #[instrument(skip(self), fields(auction.old_id = %base64(self.current_block.sequencer_block_hash())), err)]
    fn handle_optimistic_block(
        &mut self,
        optimistic_block: eyre::Result<block::Optimistic>,
    ) -> eyre::Result<()> {
        let optimistic_block = optimistic_block.wrap_err("failed receiving optimistic block")?;

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

        // forward the optimistic block to the rollup's optimistic execution server
        // TODO: don't want to exit on this, just complain with a log and skip the block or smth?
        self.blocks_to_execute_handle
            .try_send_block_to_execute(optimistic_block)
            .wrap_err("failed to send optimistic block for execution")?;

        Ok(())
    }

    #[instrument(skip_all, fields(auction.id = %base64(self.current_block.sequencer_block_hash())), err)]
    fn handle_block_commitment(&mut self, block_commitment: block::Commitment) -> eyre::Result<()> {
        // TODO: handle this with a log instead of exiting?
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

    #[instrument(skip_all, fields(auction.id = %base64(self.current_block.sequencer_block_hash())))]
    fn handle_executed_block(&mut self, executed_block: block::Executed) -> eyre::Result<()> {
        // TODO: handle this with a log instead of exiting?
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

    #[instrument(skip_all, fields(auction.id = %base64(self.current_block.sequencer_block_hash())))]
    fn handle_incoming_bundle(&mut self, bundle: Bundle) -> eyre::Result<()> {
        // TODO: use ensure! here and provide the hashes in the error
        if bundle.base_sequencer_block_hash() != self.current_block.sequencer_block_hash() {
            return Err(eyre!(
                "incoming bundle's {sequencer_block_hash} does not match current sequencer block \
                 hash"
            ));
        }

        if let Some(rollup_parent_block_hash) = self.current_block.rollup_parent_block_hash() {
            if bundle.prev_rollup_block_hash() != rollup_parent_block_hash {
                return Err(eyre!(
                    "bundle's rollup parent block hash does not match current rollup parent block \
                     hash"
                ));
            }
        } else {
            // TODO: should i buffer these up in the channel until the block is executed and then
            // filter them in the auction if the parent hashes dont match?
            return Err(eyre!("current block has not been executed yet."));
        }

        let auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());
        self.auctions
            .try_send_bundle(auction_id, bundle)
            .wrap_err("failed to submit bundle to auction")?;

        Ok(())
    }
}
