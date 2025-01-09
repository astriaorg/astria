use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::{
    primitive::v1::RollupId,
    sequencerblock::v1::block::FilteredSequencerBlock,
};
use astria_eyre::eyre::{
    self,
    OptionExt as _,
    WrapErr as _,
};
use futures::{
    stream::FuturesUnordered,
    StreamExt as _,
};
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    field,
    info,
    instrument,
    Level,
    Span,
};

use crate::{
    rollup_channel::{
        BundleStream,
        ExecuteOptimisticBlockStream,
    },
    sequencer_channel::{
        BlockCommitmentStream,
        OptimisticBlockStream,
    },
    sequencer_key::SequencerKey,
    Config,
    Metrics,
};

mod auction;

/// The implementation of the auctioneer business logic.
pub(super) struct Inner {
    auction_factory: auction::Factory,
    block_commitments: BlockCommitmentStream,
    bundles: BundleStream,
    cancelled_auctions: FuturesUnordered<auction::Auction>,
    executed_blocks: ExecuteOptimisticBlockStream,
    running_auction: Option<auction::Auction>,
    optimistic_blocks: OptimisticBlockStream,
    rollup_id: RollupId,
    shutdown_token: CancellationToken,
}

impl Inner {
    /// Creates an [`Auctioneer`] service from a [`Config`] and [`Metrics`].
    pub(super) fn new(
        config: Config,
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
        } = config;

        let rollup_id = RollupId::from_unhashed_bytes(rollup_id);
        let rollup_channel = crate::rollup_channel::open(&rollup_grpc_endpoint)?;
        let sequencer_channel = crate::sequencer_channel::open(&sequencer_grpc_endpoint)?;

        let sequencer_key = SequencerKey::builder()
            .path(sequencer_private_key_path)
            .prefix(sequencer_address_prefix)
            .try_build()
            .wrap_err("failed to load sequencer private key")?;
        info!(address = %sequencer_key.address(), "loaded sequencer signer");

        let sequencer_abci_client =
            sequencer_client::HttpClient::new(sequencer_abci_endpoint.as_str())
                .wrap_err("failed constructing sequencer abci client")?;

        // TODO: Rearchitect this thing
        let auction_factory = auction::Factory {
            metrics,
            sequencer_abci_client,
            sequencer_channel: sequencer_channel.clone(),
            latency_margin: Duration::from_millis(latency_margin_ms),
            sequencer_key: sequencer_key.clone(),
            fee_asset_denomination,
            sequencer_chain_id,
            rollup_id,
            cancellation_token: shutdown_token.child_token(),
            last_successful_nonce: None,
        };

        Ok(Self {
            auction_factory,
            block_commitments: sequencer_channel.open_get_block_commitment_stream(),
            bundles: rollup_channel.open_bundle_stream(),
            cancelled_auctions: FuturesUnordered::new(),
            executed_blocks: rollup_channel.open_execute_optimistic_block_stream(),
            optimistic_blocks: sequencer_channel.open_get_optimistic_block_stream(rollup_id),
            rollup_id,
            running_auction: None,
            shutdown_token,
        })
    }

    /// Runs the [`Auctioneer`] service until it received an exit signal, or one of the constituent
    /// tasks either ends unexpectedly or returns an error.
    pub(super) async fn run(mut self) -> eyre::Result<()> {
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

            (id, res) = async { self.running_auction.as_mut().unwrap().await }, if self.running_auction.is_some() => {
                let _ = self.handle_completed_auction(id, res);
            }

            Some(res) = self.bundles.next() => {
                let _ = self.handle_bundle(res);
            }

             Some((id, res)) = self.cancelled_auctions.next() => {
                 let _ = self.handle_cancelled_auction(id, res);
             }
        );
        Ok(())
    }

    /// Handles the result of an auction running to completion.
    ///
    /// This method exists to ensure that panicking auctions receive an event.
    /// It is assumed that auctions that ran to completion (returning a success or failure)
    /// will emit an event in their own span.
    #[instrument(skip_all, fields(%auction_id), err)]
    fn handle_completed_auction(
        &mut self,
        auction_id: auction::Id,
        res: Result<auction::Summary, auction::Error>,
    ) -> Result<auction::Summary, auction::Error> {
        if let Ok(auction::Summary::Submitted {
            nonce_used, ..
        }) = &res
        {
            self.auction_factory.set_last_successful_nonce(*nonce_used);
        }

        let _ = self.running_auction.take();
        res
    }

    /// Handles the result of cancelled auctions.
    ///
    /// This method only exists to ensure that panicking auctions receive an event.
    /// It is assumed that auctions that ran to completion (returnin a success or failure)
    /// will emit an event in their own span.
    #[instrument(skip_all, fields(%auction_id), err(level = Level::INFO))]
    fn handle_cancelled_auction(
        &self,
        auction_id: auction::Id,
        res: Result<auction::Summary, auction::Error>,
    ) -> Result<auction::Summary, auction::Error> {
        res
    }

    #[instrument(skip_all, fields(block_hash = field::Empty), err)]
    fn handle_optimistic_block(
        &mut self,
        optimistic_block: eyre::Result<FilteredSequencerBlock>,
    ) -> eyre::Result<()> {
        let optimistic_block =
            optimistic_block.wrap_err("encountered problem receiving optimistic block message")?;

        Span::current().record("block_hash", field::display(optimistic_block.block_hash()));

        let new_auction = self.auction_factory.start_new(&optimistic_block);
        info!(auction_id = %new_auction.id(), "started new auction");

        if let Some(old_auction) = self.running_auction.replace(new_auction) {
            old_auction.cancel();
            info!(auction_id = %old_auction.id(), "cancelled running auction");
            self.cancelled_auctions.push(old_auction);
        }

        // TODO: do conversion && sending in one operation
        let base_block = crate::block::Optimistic::new(optimistic_block)
            .try_into_base_block(self.rollup_id)
            // FIXME: give this their proper wire names
            .wrap_err("failed to create BaseBlock from FilteredSequencerBlock")?;
        self.executed_blocks
            .try_send(base_block)
            .wrap_err("failed to forward block to execution stream")?;

        Ok(())
    }

    #[instrument(skip_all, fields(block_hash = field::Empty), err)]
    fn handle_block_commitment(
        &mut self,
        commitment: eyre::Result<crate::block::Commitment>,
    ) -> eyre::Result<()> {
        let block_commitment = commitment.wrap_err("failed to receive block commitment")?;
        Span::current().record("block_hash", field::display(block_commitment.block_hash()));

        if let Some(running_auction) = &mut self.running_auction {
            running_auction
                .start_timer(block_commitment)
                .wrap_err("failed to start timer")?;
            info!(auction_id = %running_auction.id(), "started auction timer");
        } else {
            info!(
                "received a block commitment but did not start auction timer because no auction \
                 was running"
            );
        }

        Ok(())
    }

    #[instrument(skip_all, fields(block_hash = field::Empty), err)]
    fn handle_executed_block(
        &mut self,
        executed_block: eyre::Result<crate::block::Executed>,
    ) -> eyre::Result<()> {
        let executed_block = executed_block.wrap_err("failed to receive executed block")?;
        Span::current().record(
            "block_hash",
            field::display(executed_block.sequencer_block_hash()),
        );
        if let Some(running_auction) = &mut self.running_auction {
            running_auction
                .start_bids(executed_block)
                .wrap_err("failed to start processing bids")?;
            info!(
                auction_id = %running_auction.id(),
                "set auction to start processing bids based on executed block",
            );
        } else {
            info!(
                "received an executed block but did not set auction to start processing bids \
                 because no auction was running"
            );
        }
        Ok(())
    }

    #[instrument(skip_all, fields(block_hash = field::Empty), err)]
    fn handle_bundle(&mut self, bundle: eyre::Result<crate::bundle::Bundle>) -> eyre::Result<()> {
        let bundle = Arc::new(bundle.wrap_err("received problematic bundle")?);
        Span::current().record(
            "block_hash",
            field::display(bundle.base_sequencer_block_hash()),
        );
        if let Some(running_auction) = &mut self.running_auction {
            running_auction
                .forward_bundle_to_auction(bundle)
                .wrap_err("failed to forward bundle to auction")?;
            info!(
                auction_id = %running_auction.id(),
                "forwarded bundle to auction"
            );
        } else {
            info!(
                "received a bundle but did not forward it to the auction because no auction was \
                 running",
            );
        }
        Ok(())
    }

    #[instrument(skip_all)]
    async fn shutdown(mut self, reason: eyre::Result<&'static str>) -> eyre::Result<()> {
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
        if let Some(running_auction) = self.running_auction.take() {
            running_auction.abort();
        }
        reason.map(|_| ())
    }
}
