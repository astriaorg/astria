use std::time::Duration;

use astria_core::{
    primitive::v1::RollupId,
    sequencerblock::v1::block::FilteredSequencerBlock,
};
use astria_eyre::eyre::{
    self,
    OptionExt as _,
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
    rollup_channel::{
        BundleStream,
        ExecuteOptimisticBlockStream,
        RollupChannel,
    },
    sequencer_channel::{
        BlockCommitmentStream,
        OptimisticBlockStream,
        SequencerChannel,
    },
    Config,
    Metrics,
};

macro_rules! break_for_closed_stream {
    ($stream_res:expr, $msg:expr) => {
        match $stream_res {
            Some(val) => val,
            None => break Err(::astria_eyre::eyre::eyre!($msg)),
        }
    };
}

/// The implementation of the auctioneer business logic.
pub(super) struct Inner {
    /// Used to signal the service to shutdown
    run_state: RunState,
}

impl Inner {
    /// Creates an [`Auctioneer`] service from a [`Config`] and [`Metrics`].
    pub(super) fn new(
        cfg: Config,
        metrics: &'static Metrics,
        shutdown_token: CancellationToken,
    ) -> eyre::Result<Self> {
        let Config {
            sequencer_grpc_endpoint,
            sequencer_abci_endpoint,
            latency_margin_ms,
            rollup_grpc_endpoint,
            rollup_id,
            sequencer_chain_id,
            sequencer_private_key_path,
            sequencer_address_prefix,
            fee_asset_denomination,
            ..
        } = cfg;

        let rollup_channel = crate::rollup_channel::open(&rollup_grpc_endpoint)?;
        let sequencer_channel = crate::sequencer_channel::open(&sequencer_grpc_endpoint)?;

        // TODO: Rearchitect this thing
        let auctions = auction::manager::Builder {
            metrics,
            shutdown_token: shutdown_token.clone(),
            sequencer_grpc_endpoint: sequencer_grpc_endpoint.clone(),
            sequencer_abci_endpoint,
            latency_margin: Duration::from_millis(latency_margin_ms),
            sequencer_private_key_path,
            sequencer_address_prefix,
            fee_asset_denomination,
            sequencer_chain_id,
            rollup_id: rollup_id.clone(),
        }
        .build()
        .wrap_err("failed to initialize auction manager")?;

        Ok(Self {
            run_state: Starting {
                auctions,
                rollup_channel,
                rollup_id: RollupId::from_unhashed_bytes(&rollup_id),
                sequencer_channel,
                shutdown_token,
            }
            .into(),
        })
    }

    /// Runs the [`Auctioneer`] service until it received an exit signal, or one of the constituent
    /// tasks either ends unexpectedly or returns an error.
    pub(super) async fn run(self) -> eyre::Result<()> {
        let Self {
            mut run_state,
        } = self;

        loop {
            match run_state {
                RunState::Cancelled => break Ok(()),
                RunState::Starting(starting) => match starting.run().await {
                    Ok(new_state) => run_state = new_state,
                    Err(err) => break Err(err).wrap_err("failed during startup"),
                },
                RunState::Running(running) => match running.run().await {
                    Ok(new_state) => run_state = new_state,
                    Err(err) => break Err(err).wrap_err("failed during execution"),
                },
            }
        }
    }
}

enum RunState {
    Cancelled,
    Starting(Starting),
    Running(Running),
}

impl From<Running> for RunState {
    fn from(value: Running) -> Self {
        Self::Running(value)
    }
}

impl From<Starting> for RunState {
    fn from(value: Starting) -> Self {
        Self::Starting(value)
    }
}

struct Starting {
    auctions: auction::Manager,
    rollup_channel: RollupChannel,
    rollup_id: RollupId,
    sequencer_channel: SequencerChannel,
    shutdown_token: CancellationToken,
}

impl Starting {
    async fn run(self) -> eyre::Result<RunState> {
        let Self {
            auctions,
            rollup_id,
            rollup_channel,
            mut sequencer_channel,
            shutdown_token,
        } = self;

        let executed_blocks = rollup_channel
            .open_execute_optimistic_block_stream()
            .await
            .wrap_err("opening stream to execute optimistic blocks on rollup failed")?;

        let mut optimistic_blocks = sequencer_channel
            .open_get_optimistic_block_stream(rollup_id)
            .await
            .wrap_err("opening stream to receive optimistic blocks from sequencer failed")?;

        let block_commitments = sequencer_channel
            .open_get_block_commitment_stream()
            .await
            .wrap_err("opening stream to receive block commitments from sequencer failed")?;

        let bundles = rollup_channel
            .open_bundle_stream()
            .await
            .wrap_err("opening stream to receive bundles from rollup failed")?;

        let optimistic_block = optimistic_blocks
            .next()
            .await
            .ok_or_eyre("optimistic stream closed during startup?")?
            .wrap_err("failed to get optimistic block during startup")?;
        let current_block = crate::block::Current::with_optimistic(optimistic_block);

        Ok(Running {
            auctions,
            block_commitments,
            bundles,
            current_block,
            executed_blocks,
            optimistic_blocks,
            rollup_id,
            shutdown_token,
        }
        .into())
    }
}

struct Running {
    auctions: auction::Manager,
    block_commitments: BlockCommitmentStream,
    bundles: BundleStream,
    current_block: crate::block::Current,
    executed_blocks: ExecuteOptimisticBlockStream,
    optimistic_blocks: OptimisticBlockStream,
    rollup_id: RollupId,
    shutdown_token: CancellationToken,
}

impl Running {
    async fn run(mut self) -> eyre::Result<RunState> {
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

                    Some(res) = self.bundles.next() => {
                        let bundle = res.wrap_err("failed to get bundle")?;

                        let _ = self.handle_bundle(bundle);
                    }
                }
            }
        };

        self.shutdown(reason)
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

        self.current_block = crate::block::Current::with_optimistic(optimistic_block.clone());
        let new_auction_id =
            auction::Id::from_sequencer_block_hash(self.current_block.sequencer_block_hash());
        self.auctions.new_auction(new_auction_id);

        let base_block = crate::block::Optimistic::new(optimistic_block)
            .try_into_base_block(self.rollup_id)
            // FIXME: give this their proper wire names
            .wrap_err("failed to create BaseBlock from FilteredSequencerBlock")?;
        self.executed_blocks
            .try_send(base_block)
            .wrap_err("failed to forward block to execution stream")?;

        Ok(())
    }

    #[instrument(skip_all, fields(auction.id = %base64(self.current_block.sequencer_block_hash())), err)]
    fn handle_block_commitment(
        &mut self,
        block_commitment: eyre::Result<crate::block::Commitment>,
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
        executed_block: eyre::Result<crate::block::Executed>,
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
    fn handle_bundle(&mut self, bundle: crate::bundle::Bundle) -> eyre::Result<()> {
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

    #[instrument(skip_all)]
    fn shutdown(self, reason: eyre::Result<&'static str>) -> eyre::Result<RunState> {
        let message: &str = "shutting down";
        match reason {
            Ok(reason) => info!(%reason, message),
            Err(reason) => error!(%reason, message),
        };
        Ok(RunState::Cancelled)
    }
}
