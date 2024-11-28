/// The auction Manager is responsible for managing running auction futures and their
/// associated handles.
use astria_core::{
    primitive::v1::{
        asset,
        RollupId,
    },
    sequencerblock::v1::block::FilteredSequencerBlock,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::task::JoinHandle;
use tracing::{
    info,
    instrument,
    warn,
};

use super::{
    Bundle,
    SequencerKey,
};
use crate::{
    auctioneer::PendingNonceSubscriber,
    block::Commitment,
    flatten_join_result,
};

pub(crate) struct Builder {
    pub(crate) metrics: &'static crate::Metrics,

    /// The ABCI endpoint for the sequencer service used by auctions.
    pub(crate) sequencer_abci_endpoint: String,
    /// The amount of time to run the auction timer for.
    pub(crate) latency_margin: std::time::Duration,
    /// The private key used to sign sequencer transactions.
    pub(crate) sequencer_key: SequencerKey,
    /// The denomination of the fee asset used in the sequencer transactions
    pub(crate) fee_asset_denomination: asset::Denom,
    /// The chain ID for sequencer transactions
    pub(crate) sequencer_chain_id: String,
    /// The rollup ID for the `RollupDataSubmission`s with auction results
    pub(crate) rollup_id: RollupId,
    pub(crate) pending_nonce: PendingNonceSubscriber,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<Manager> {
        let Self {
            metrics,
            sequencer_abci_endpoint,
            latency_margin,
            fee_asset_denomination,
            rollup_id,
            sequencer_key,
            sequencer_chain_id,
            pending_nonce,
        } = self;

        let sequencer_abci_client =
            sequencer_client::HttpClient::new(sequencer_abci_endpoint.as_str())
                .wrap_err("failed constructing sequencer abci client")?;

        Ok(Manager {
            metrics,
            sequencer_abci_client,
            latency_margin,
            running_auction: None,
            sequencer_key,
            fee_asset_denomination,
            sequencer_chain_id,
            rollup_id,
            pending_nonce,
        })
    }
}

struct RunningAuction {
    id: crate::auction::Id,
    height: u64,
    parent_block_of_executed: Option<[u8; 32]>,
    // TODO: Rename this to AuctionSender or smth like that
    sender: crate::auction::Handle,
    task: JoinHandle<eyre::Result<()>>,
}

impl RunningAuction {
    fn abort(&self) {
        self.task.abort()
    }
}

pub(crate) struct Manager {
    #[allow(dead_code)]
    metrics: &'static crate::Metrics,
    sequencer_abci_client: sequencer_client::HttpClient,
    latency_margin: std::time::Duration,
    running_auction: Option<RunningAuction>,
    sequencer_key: SequencerKey,
    fee_asset_denomination: asset::Denom,
    sequencer_chain_id: String,
    rollup_id: RollupId,
    pending_nonce: PendingNonceSubscriber,
}

impl Manager {
    // pub(crate) fn new_auction(&mut self, auction_id: Id) {
    // TODO: Add some better instrumentation.
    #[instrument(skip(self))]
    pub(crate) fn new_auction(&mut self, block: FilteredSequencerBlock) {
        let new_auction_id = crate::auction::Id::from_sequencer_block_hash(*block.block_hash());
        let height = block.height().into();

        if let Some(running_auction) = self.running_auction.take() {
            // NOTE: We just throw away the old auction after aborting it. Is there
            // value in `.join`ing it after the abort except for ensuring that it
            // did indeed abort? What if the running auction is not tracked inside
            // the "auction manager", but the auction manager is turned into a simpler
            // factory so that the running auction is running inside the auctioneer?
            // Then the auctioneer would always have the previous/old auction and could
            // decide what to do with it. That might make this a cleaner implementation.
            let old_auction_id = running_auction.id;
            info!(
                %new_auction_id,
                %old_auction_id,
                "received optimistic block, aborting old auction and starting new auction"
            );

            running_auction.abort();
        }

        let (handle, auction) = super::Builder {
            sequencer_abci_client: self.sequencer_abci_client.clone(),
            latency_margin: self.latency_margin,
            auction_id: new_auction_id,
            sequencer_key: self.sequencer_key.clone(),
            fee_asset_denomination: self.fee_asset_denomination.clone(),
            sequencer_chain_id: self.sequencer_chain_id.clone(),
            rollup_id: self.rollup_id,
            pending_nonce: self.pending_nonce.clone(),
        }
        .build();

        self.running_auction = Some(RunningAuction {
            id: new_auction_id,
            height,
            parent_block_of_executed: None,
            sender: handle,
            task: tokio::task::spawn(auction.run()),
        });
    }

    #[instrument(skip(self))]
    // pub(crate) fn start_timer(&mut self, auction_id: Id) -> eyre::Result<()> {
    pub(crate) fn start_timer(&mut self, block_commitment: Commitment) -> eyre::Result<()> {
        let id_according_to_block =
            crate::auction::Id::from_sequencer_block_hash(block_commitment.sequencer_block_hash());

        if let Some(auction) = &mut self.running_auction {
            if auction.id == id_according_to_block
                && block_commitment.sequencer_height() == auction.height
            {
                auction
                    .sender
                    .start_timer()
                    .wrap_err("failed to send command to start timer to auction")?;
            } else {
                // TODO: provide better information on the blocks/currently running auction.
                // warn!(
                //     current_block.sequencer_block_hash =
                // %base64(self.current_block.sequencer_block_hash()),
                //     block_commitment.sequencer_block_hash =
                // %base64(block_commitment.sequencer_block_hash()),     "received
                // block commitment for the wrong block" );
                info!(
                    "not starting the auction timer because sequencer block hash and height of \
                     the commitment did not match that of the running auction",
                );
            }
        } else {
            info!(
                "cannot start the auction timer with the received executed block because no \
                 auction was currently running; dropping the commit block",
            );
        }

        Ok(())
    }

    #[instrument(skip(self))]
    // pub(crate) fn start_processing_bids(&mut self, auction_id: Id) -> eyre::Result<()> {
    pub(crate) fn start_processing_bids(
        &mut self,
        block: crate::block::Executed,
    ) -> eyre::Result<()> {
        let id_according_to_block =
            crate::auction::Id::from_sequencer_block_hash(block.sequencer_block_hash());

        if let Some(auction) = &mut self.running_auction {
            if auction.id == id_according_to_block {
                // TODO: What if it was already set? Overwrite? Replace? Drop?
                let _ = auction
                    .parent_block_of_executed
                    .replace(block.parent_rollup_block_hash());
                auction
                    .sender
                    .start_processing_bids()
                    .wrap_err("failed to send command to start processing bids")?;
            } else {
                // TODO: bring back the fields to track the dropped block and current block
                // warn!(
                //     // TODO: nicer display for the current block
                //     current_block.sequencer_block_hash =
                // %base64(self.current_block.sequencer_block_hash()),
                //     executed_block.sequencer_block_hash =
                // %base64(executed_block.sequencer_block_hash()),
                //     executed_block.rollup_block_hash =
                // %base64(executed_block.rollup_block_hash()),     "received
                // optimistic execution result for wrong sequencer block" );
                warn!(
                    "not starting to process bids in the current auction because we received an \
                     executed block from the rollup with a sequencer block hash that does not \
                     match that of the currently running auction; dropping the executed block"
                );
            }
        } else {
            info!(
                "cannot start processing bids with the received executed block because no auction \
                 was currently running; dropping the executed block"
            );
        }
        Ok(())
    }

    pub(crate) fn forward_bundle_to_auction(&mut self, bundle: Bundle) -> eyre::Result<()> {
        let id_according_to_bundle =
            crate::auction::Id::from_sequencer_block_hash(bundle.base_sequencer_block_hash());
        if let Some(auction) = &mut self.running_auction {
            // TODO: remember to check the parent rollup block hash, i.e.:
            //
            // if let Some(bundle.rollup_parent_block_hash) =
            // current_auction/block.parent_rollup_block_hash() {     ensure!(
            //         bundle.parent_rollup_block_hash() == rollup_parent_block_hash,
            //         "bundle's rollup parent block hash {bundle_hash} does not match current
            // rollup \          parent block hash {current_hash}",
            //         bundle_hash = base64(bundle.parent_rollup_block_hash()),
            //         current_hash = base64(rollup_parent_block_hash)
            //     );
            // }
            let Some(parent_block_of_executed) = auction.parent_block_of_executed else {
                eyre::bail!(
                    "received a new bundle but the current auction has not yet
                    received an execute block from the rollup; dropping the bundle"
                );
            };
            let ids_match = auction.id == id_according_to_bundle;
            let parent_blocks_match = parent_block_of_executed == bundle.parent_rollup_block_hash();
            if ids_match && parent_blocks_match {
                auction
                    .sender
                    .try_send_bundle(bundle)
                    .wrap_err("failed to add bundle to auction")?;
            } else {
                warn!(
                    // TODO: Add these fields back in. Is it even necessary to return the error?
                    // Can't we just fire the event here? necessary?
                    //
                    // curent_block.sequencer_block_hash = %base64(self.
                    // current_block.sequencer_block_hash()),
                    // bundle.sequencer_block_hash = %base64(bundle.base_sequencer_block_hash()),
                    // bundle.parent_rollup_block_hash =
                    // %base64(bundle.parent_rollup_block_hash()),
                    "incoming bundle does not match current block, ignoring"
                );
                eyre::bail!("auction ID and ID according to bundle don't match");
            }
        } else {
            info!(
                "cannot forward the received bundle to an auction because no auction is currently \
                 running; dropping the bundle"
            );
        }
        Ok(())
    }

    pub(crate) async fn next_winner(&mut self) -> Option<eyre::Result<()>> {
        let auction = self.running_auction.as_mut()?;
        Some(flatten_join_result((&mut auction.task).await))
    }

    pub(crate) fn abort(&mut self) {
        // TODO: Do we need to wait for it to finish?
        if let Some(auction) = self.running_auction.take() {
            auction.abort()
        }
    }
}
