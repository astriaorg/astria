//! The Optimistic Executor is the component responsible for maintaining the current block
//! state based on the optimistic block stream, the block commitment stream, and the executed
//! block stream. The Optimistic Executior uses its current block state for running an auction
//! per block. Incoming bundles are fed to the current auction after checking them against the
//! current block state.
//!
//! ## Block Lifecycle
//! The Optimistic Executor tracks its current block using the `block:Current` struct, at a high
//! level:
//! 1. Blocks are received optimistically from the sequencer via the optimistic block stream, which
//!    also forwards them to the rollup node for execution.
//! 2. Execution results are received from the rollup node via the executed block stream.
//! 3. Commitments are received from the sequencer via the block commitment stream.
//!
//! ## Auction Lifecycle
//! The current block state is used for running an auction per block. Auctions are managed by the
//! `auction::Manager` struct, and the Optimistic Executor advances their state based on its current
//! block state:
//! 1. An auction is created when a new block is received optimistically.
//! 2. The auction will begin processing bids when the executed block is received.
//! 3. The auction's timer is started when a block commitment is received.
//! 4. Bundles are fed to the current auction after checking them against the current block state.
//!
//! ### Bundles and Backpressure
//! Bundles are fed to the current auction via an mpsc channel. Since the auction will only start
//! processing bids after the executed block is received, the channel is used to buffer bundles
//! that are received before the executed block.
//! If too many bundles are received from the rollup node before the block is executed
//! optimistically on the rollup node, the channel will fill up and newly received bundles will be
//! dropped until the auction begins processing bundles.
//! We assume this is highly unlikely, as the rollup node's should filter the bundles it streams
//! by its optimistic head block hash.

use astria_core::{
    primitive::v1::RollupId,
    sequencerblock::v1::block::FilteredSequencerBlock,
};
use astria_eyre::eyre::{
    self,
    eyre,
    OptionExt,
    WrapErr as _,
};
use futures::StreamExt as _;
use telemetry::display::base64;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    instrument,
    warn,
};

use crate::{
    auction,
    block::{
        self,
        executed_stream::ExecutedBlockStream,
    },
    bundle::{
        Bundle,
        BundleStream,
    },
    sequencer_channel::{
        BlockCommitmentStream,
        OptimisticBlockStream,
        SequencerChannel,
    },
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
    sequencer_channel: SequencerChannel,
    rollup_id: RollupId,
    rollup_grpc_endpoint: String,
    auctions: auction::Manager,
}

impl Startup {
    pub(crate) async fn startup(self) -> eyre::Result<Running> {
        let Self {
            metrics,
            shutdown_token,
            mut sequencer_channel,
            rollup_id,
            rollup_grpc_endpoint,
            auctions,
        } = self;

        let (execution_stream_handle, executed_blocks) =
            ExecutedBlockStream::connect(rollup_id, rollup_grpc_endpoint.clone())
                .await
                .wrap_err("failed to initialize executed block stream")?;

        let mut optimistic_blocks = sequencer_channel
            .open_get_optimistic_block_stream(rollup_id)
            .await
            .wrap_err("opening stream to receive optimistic blocks from sequencer failed")?;

        // TODO: create a way to forward the optimistic blocks to the execution stream.

        let block_commitments = sequencer_channel
            .open_get_block_commitment_stream()
            .await
            .wrap_err("opening stream to receive block commitments from sequencer failed")?;

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
            bundle_stream,
            auctions,
            current_block,
            execution_stream_handle,
        })
    }
}

pub(crate) struct Running {
    // TODO: add metrics
    #[allow(dead_code)]
    metrics: &'static crate::Metrics,
    shutdown_token: CancellationToken,
    optimistic_blocks: OptimisticBlockStream,
    block_commitments: BlockCommitmentStream,
    executed_blocks: ExecutedBlockStream,
    bundle_stream: BundleStream,
    auctions: auction::Manager,
    current_block: block::Current,
    execution_stream_handle: crate::block::executed_stream::Handle,
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
                        res.wrap_err_with(|| format!("auction failed for block {}", base64(id)))?;
                    },

                    res = self.optimistic_blocks.next() => {
                        let res = break_for_closed_stream!(res, "optimistic block stream closed");
                        let _ = self.handle_optimistic_block(res);
                    },

                    res = self.block_commitments.next() => {
                        let res = break_for_closed_stream!(res, "block commitment stream closed");

                        let _ = self.handle_block_commitment(res);

                    },

                    res = self.executed_blocks.next() => {
                        let res = break_for_closed_stream!(res, "executed block stream closed");

                        let _ = self.handle_executed_block(res);
                    }

                    Some(res) = self.bundle_stream.next() => {
                        let bundle = res.wrap_err("failed to get bundle")?;

                        let _ = self.handle_bundle(bundle);
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
        optimistic_block: eyre::Result<FilteredSequencerBlock>,
    ) -> eyre::Result<()> {
        let optimistic_block = optimistic_block.wrap_err("failed to receive optimistic block")?;

        let old_auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());
        self.auctions
            .abort_auction(old_auction_id)
            .wrap_err("failed to abort auction")?;

        info!(
            optimistic_block.sequencer_block_hash = %base64(optimistic_block.block_hash()),
            "received optimistic block, aborting old auction and starting new auction"
        );

        self.current_block = block::Current::with_optimistic(optimistic_block.clone());
        let new_auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());
        self.auctions.new_auction(new_auction_id);

        self.execution_stream_handle
            .try_send_block_to_execute(optimistic_block)
            .wrap_err("failed to forward block to execution stream")?;

        Ok(())
    }

    #[instrument(skip_all, fields(auction.id = %base64(self.current_block.sequencer_block_hash())), err)]
    fn handle_block_commitment(
        &mut self,
        block_commitment: eyre::Result<block::Commitment>,
    ) -> eyre::Result<()> {
        let block_commitment = block_commitment.wrap_err("failed to receive block commitment")?;

        if let Err(e) = self.current_block.commitment(block_commitment.clone()) {
            warn!(
                current_block.sequencer_block_hash = %base64(self.current_block.sequencer_block_hash()),
                block_commitment.sequencer_block_hash = %base64(block_commitment.sequencer_block_hash()),
                "received block commitment for the wrong block"
            );
            return Err(e).wrap_err("failed to handle block commitment");
        }

        let auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());

        self.auctions
            .start_timer(auction_id)
            .wrap_err("failed to start timer")?;

        Ok(())
    }

    #[instrument(skip_all, fields(auction.id = %base64(self.current_block.sequencer_block_hash())))]
    fn handle_executed_block(
        &mut self,
        executed_block: eyre::Result<block::Executed>,
    ) -> eyre::Result<()> {
        let executed_block = executed_block.wrap_err("failed to receive executed block")?;

        if let Err(e) = self.current_block.execute(executed_block.clone()) {
            warn!(
                // TODO: nicer display for the current block
                current_block.sequencer_block_hash = %base64(self.current_block.sequencer_block_hash()),
                executed_block.sequencer_block_hash = %base64(executed_block.sequencer_block_hash()),
                executed_block.rollup_block_hash = %base64(executed_block.rollup_block_hash()),
                "received optimistic execution result for wrong sequencer block"
            );
            return Err(e).wrap_err("failed to handle executed block");
        }

        let auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());

        self.auctions
            .start_processing_bids(auction_id)
            .wrap_err("failed to start processing bids")?;

        Ok(())
    }

    #[instrument(skip_all, fields(auction.id = %base64(self.current_block.sequencer_block_hash())))]
    fn handle_bundle(&mut self, bundle: Bundle) -> eyre::Result<()> {
        if let Err(e) = self.current_block.ensure_bundle_is_valid(&bundle) {
            warn!(
                curent_block.sequencer_block_hash = %base64(self.current_block.sequencer_block_hash()),
                bundle.sequencer_block_hash = %base64(bundle.base_sequencer_block_hash()),
                bundle.parent_rollup_block_hash = %base64(bundle.parent_rollup_block_hash()),
                "incoming bundle does not match current block, ignoring"
            );
            return Err(e).wrap_err("failed to handle bundle");
        }

        let auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());
        self.auctions
            .try_send_bundle(auction_id, bundle)
            .wrap_err("failed to submit bundle to auction")?;

        Ok(())
    }
}
