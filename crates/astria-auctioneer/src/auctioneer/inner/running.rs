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
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    instrument,
};

use super::RunState;
use crate::{
    rollup_channel::{
        BundleStream,
        ExecuteOptimisticBlockStream,
    },
    sequencer_channel::{
        BlockCommitmentStream,
        OptimisticBlockStream,
    },
};

pub(super) struct Running {
    pub(super) auctions: crate::auction::Manager,
    pub(super) block_commitments: BlockCommitmentStream,
    pub(super) bundles: BundleStream,
    pub(super) executed_blocks: ExecuteOptimisticBlockStream,
    pub(super) optimistic_blocks: OptimisticBlockStream,
    pub(super) rollup_id: RollupId,
    pub(super) shutdown_token: CancellationToken,
}

impl Running {
    pub(super) async fn run(mut self) -> eyre::Result<RunState> {
        let reason: eyre::Result<&str> = {
            // This is a long running loop. Errors are emitted inside the handlers.
            loop {
                select! {
                    biased;

                    () = self.shutdown_token.clone().cancelled_owned() => {
                        break Ok("received shutdown signal");
                    },

                    res = self.handle_event() => {
                        if let Err(err) = res {
                            break Err(err);
                        }
                    }
                }
            }
        };

        self.shutdown(reason).await
    }

    async fn handle_event(&mut self) -> eyre::Result<()> {
        select!(
            res = self.optimistic_blocks.next() => {
                let res = res.ok_or_eyre("optimistic block stream closed")?;
                let _ = self.handle_optimistic_block(res);
            },

            res = self.block_commitments.next() => {
                let res = res.ok_or_eyre("block commitment stream closed")?;
                let _ = self.handle_block_commitment(res);
            },

            res = self.executed_blocks.next() => {
                let res = res.ok_or_eyre("executed block stream closed")?;
                let _ = self.handle_executed_block(res);
            }

            Some(res) = self.auctions.next_winner() => {
                let _ = self.handle_auction_winner(res);
            }

            Some(res) = self.bundles.next() => {
                let _ = self.handle_bundle(res);
            }
        );
        Ok(())
    }

    #[instrument(skip_all, err)]
    fn handle_auction_winner(&self, res: eyre::Result<()>) -> eyre::Result<()> {
        res
    }

    // #[instrument(skip(self), fields(auction.old_id =
    // %base64(self.current_block.sequencer_block_hash())), err)]
    fn handle_optimistic_block(
        &mut self,
        optimistic_block: eyre::Result<FilteredSequencerBlock>,
    ) -> eyre::Result<()> {
        let optimistic_block =
            optimistic_block.wrap_err("encountered problem receiving optimistic block message")?;

        // FIXME: Don't clone this; find a better way.
        self.auctions.new_auction(optimistic_block.clone());

        let base_block = crate::block::Optimistic::new(optimistic_block)
            .try_into_base_block(self.rollup_id)
            // FIXME: give this their proper wire names
            .wrap_err("failed to create BaseBlock from FilteredSequencerBlock")?;
        self.executed_blocks
            .try_send(base_block)
            .wrap_err("failed to forward block to execution stream")?;

        Ok(())
    }

    // #[instrument(skip_all, fields(auction.id =
    // %base64(self.current_block.sequencer_block_hash())), err)]
    fn handle_block_commitment(
        &mut self,
        block_commitment: eyre::Result<crate::block::Commitment>,
    ) -> eyre::Result<()> {
        let block_commitment = block_commitment.wrap_err("failed to receive block commitment")?;

        self.auctions
            .start_timer(block_commitment)
            .wrap_err("failed to start timer")?;

        Ok(())
    }

    // #[instrument(skip_all, fields(auction.id =
    // %base64(self.current_block.sequencer_block_hash())))]
    fn handle_executed_block(
        &mut self,
        executed_block: eyre::Result<crate::block::Executed>,
    ) -> eyre::Result<()> {
        let executed_block = executed_block.wrap_err("failed to receive executed block")?;
        self.auctions
            .start_processing_bids(executed_block)
            .wrap_err("failed to start processing bids")?;
        Ok(())
    }

    // #[instrument(skip_all, fields(auction.id =
    // %base64(self.current_block.sequencer_block_hash())))]
    fn handle_bundle(&mut self, bundle: eyre::Result<crate::bundle::Bundle>) -> eyre::Result<()> {
        let bundle = bundle.wrap_err("received problematic bundle")?;
        self.auctions
            .forward_bundle_to_auction(bundle)
            .wrap_err("failed to forward bundle to auction")?;
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
        self.auctions.abort();
        reason.map(|_| RunState::Cancelled)
    }
}
