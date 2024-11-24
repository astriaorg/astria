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

use super::inner::RunState;
use crate::{
    auction,
    rollup_channel::{
        BundleStream,
        ExecuteOptimisticBlockStream,
    },
    sequencer_channel::{
        BlockCommitmentStream,
        OptimisticBlockStream,
    },
};

/// To break from a loop instead of using the `?` operator.
macro_rules! try_break {
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(err) => break Err(err),
        }
    };
}

pub(super) struct Running {
    pub(in crate::auctioneer) auctions: crate::auction::Manager,
    pub(in crate::auctioneer) block_commitments: BlockCommitmentStream,
    pub(in crate::auctioneer) bundles: BundleStream,
    pub(in crate::auctioneer) current_block: crate::block::Current,
    pub(in crate::auctioneer) executed_blocks: ExecuteOptimisticBlockStream,
    pub(in crate::auctioneer) optimistic_blocks: OptimisticBlockStream,
    pub(in crate::auctioneer) rollup_id: RollupId,
    pub(in crate::auctioneer) shutdown_token: CancellationToken,
}

impl Running {
    pub(super) async fn run(mut self) -> eyre::Result<RunState> {
        let reason: eyre::Result<&str> = {
            // This is a long running loop. Errors are emitted inside the handlers.
            loop {
                select! {
                    biased;

                    () = self.shutdown_token.cancelled() => {
                        break Ok("received shutdown signal");
                    },

                    Some((id, res)) = self.auctions.join_next() => {
                        try_break!(res.wrap_err_with(|| format!("auction failed for block `{}`", base64(id))));
                    },

                    res = self.optimistic_blocks.next() => {
                        let res = try_break!(res.ok_or_eyre("optimistic block stream closed"));
                        let _ = self.handle_optimistic_block(res);
                    },

                    res = self.block_commitments.next() => {
                        let res = try_break!(res.ok_or_eyre("block commitment stream closed"));
                        let _ = self.handle_block_commitment(res);
                    },

                    res = self.executed_blocks.next() => {
                        let res = try_break!(res.ok_or_eyre("executed block stream closed"));
                        let _ = self.handle_executed_block(res);
                    }

                    Some(res) = self.bundles.next() => {
                        let bundle = res.wrap_err("failed to get bundle")?;
                        let _ = self.handle_bundle(bundle);
                    }
                }
            }
        };

        self.shutdown(reason).await
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
    async fn shutdown(mut self, reason: eyre::Result<&'static str>) -> eyre::Result<RunState> {
        const WAIT_BEFORE_ABORT: Duration = Duration::from_secs(25);

        // Necessary if we got here because of another reason than receiving an external
        // shutdown signal.
        self.shutdown_token.cancel();

        let message = format!(
            "waiting {} for all constituent tasks to shutdown before aborting",
            humantime::format_duration(WAIT_BEFORE_ABORT),
        );
        match &reason {
            Ok(reason) => info!(%reason, message),
            Err(reason) => error!(%reason, message),
        };
        let shutdown_auctions = async {
            while let Some((id, res)) = self.auctions.join_next().await {
                if let Err(error) = res {
                    // FIXME: probide a display impl for this ID
                    warn!(?id, %error, "auction ended with an error");
                }
            }
        };
        // NOTE: we don't care if this elapses. We will abort all auctions anyway
        // and report if there were any still running.
        let _ = tokio::time::timeout(WAIT_BEFORE_ABORT, shutdown_auctions).await;
        let aborted = self.auctions.abort_all();
        if aborted > 0 {
            warn!("aborted `{aborted}` auctions still running after grace period",);
        }
        reason.map(|_| RunState::Cancelled)
    }
}
