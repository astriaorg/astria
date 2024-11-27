/// The auction Manager is responsible for managing running auction futures and their
/// associated handles.
use std::collections::HashMap;

use astria_core::{
    generated::sequencerblock::v1::sequencer_service_client::SequencerServiceClient,
    primitive::v1::{
        asset,
        RollupId,
    },
    sequencerblock::v1::block::FilteredSequencerBlock,
};
use astria_eyre::eyre::{
    self,
    OptionExt as _,
    WrapErr as _,
};
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tonic::transport::Channel;
use tracing::{
    info,
    instrument,
    warn,
};

use super::{
    Bundle,
    Handle,
    Id,
    SequencerKey,
};
use crate::{
    block::Commitment,
    flatten_join_result,
};

pub(crate) struct Builder {
    pub(crate) metrics: &'static crate::Metrics,
    pub(crate) shutdown_token: CancellationToken,

    /// The gRPC endpoint for the sequencer service used by auctions.
    pub(crate) sequencer_grpc_client: SequencerServiceClient<Channel>,
    /// The ABCI endpoint for the sequencer service used by auctions.
    pub(crate) sequencer_abci_endpoint: String,
    /// The amount of time to run the auction timer for.
    pub(crate) latency_margin: std::time::Duration,
    /// The private key used to sign sequencer transactions.
    pub(crate) sequencer_private_key_path: String,
    /// The prefix for the address used to sign sequencer transactions
    pub(crate) sequencer_address_prefix: String,
    /// The denomination of the fee asset used in the sequencer transactions
    pub(crate) fee_asset_denomination: asset::Denom,
    /// The chain ID for sequencer transactions
    pub(crate) sequencer_chain_id: String,
    /// The rollup ID for the `RollupDataSubmission`s with auction results
    pub(crate) rollup_id: RollupId,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<Manager> {
        let Self {
            metrics,
            shutdown_token,
            sequencer_grpc_client,
            sequencer_abci_endpoint,
            latency_margin,
            fee_asset_denomination,
            rollup_id,
            sequencer_private_key_path,
            sequencer_address_prefix,
            sequencer_chain_id,
        } = self;

        let sequencer_key = SequencerKey::builder()
            .path(sequencer_private_key_path)
            .prefix(sequencer_address_prefix)
            .try_build()
            .wrap_err("failed to load sequencer private key")?;
        info!(address = %sequencer_key.address(), "loaded sequencer signer");

        let sequencer_abci_client =
            sequencer_client::HttpClient::new(sequencer_abci_endpoint.as_str())
                .wrap_err("failed constructing sequencer abci client")?;

        Ok(Manager {
            metrics,
            cancellation_token: shutdown_token,
            sequencer_grpc_client,
            sequencer_abci_client,
            latency_margin,
            running_auctions: JoinMap::new(),
            auction_handles: HashMap::new(),
            sequencer_key,
            fee_asset_denomination,
            sequencer_chain_id,
            rollup_id,

            current_block: None,
        })
    }
}

pub(crate) struct Manager {
    metrics: &'static crate::Metrics,
    cancellation_token: CancellationToken,
    sequencer_grpc_client: SequencerServiceClient<tonic::transport::Channel>,
    sequencer_abci_client: sequencer_client::HttpClient,
    latency_margin: std::time::Duration,
    // FIXME: Having a joinmap here is actually weird: if a new block arrives
    // the old auction should always be nuked. Either the optimistic block was
    // replaced (proposed block rejected), or a new block is being built (auctioneer
    // failed to submit the winning allocation in time).
    running_auctions: JoinMap<Id, eyre::Result<()>>,
    auction_handles: HashMap<Id, Handle>,
    sequencer_key: SequencerKey,
    fee_asset_denomination: asset::Denom,
    sequencer_chain_id: String,
    rollup_id: RollupId,
    current_block: Option<crate::block::Current>,
}

impl Manager {
    // pub(crate) fn new_auction(&mut self, auction_id: Id) {
    // TODO: Add some better instrumentation.
    #[instrument(skip(self))]
    pub(crate) fn new_auction(&mut self, block: FilteredSequencerBlock) {
        let new_auction_id = crate::auction::Id::from_sequencer_block_hash(*block.block_hash());

        if let Some(old_block) = self
            .current_block
            .replace(crate::block::Current::with_optimistic(block))
        {
            // TODO: Track the ID in the "current block" (or get rid of it altogether?)
            let old_auction_id =
                crate::auction::Id::from_sequencer_block_hash(old_block.sequencer_block_hash());
            info!(
                %new_auction_id,
                %old_auction_id,
                "received optimistic block, aborting old auction and starting new auction"
            );

            // TODO: provide feedback if the auction didn't exist?;
            let _ = self.abort_auction(old_auction_id);
        }

        let (handle, auction) = super::Builder {
            metrics: self.metrics,
            cancellation_token: self.cancellation_token.child_token(),
            sequencer_grpc_client: self.sequencer_grpc_client.clone(),
            sequencer_abci_client: self.sequencer_abci_client.clone(),
            latency_margin: self.latency_margin,
            auction_id: new_auction_id,
            sequencer_key: self.sequencer_key.clone(),
            fee_asset_denomination: self.fee_asset_denomination.clone(),
            sequencer_chain_id: self.sequencer_chain_id.clone(),
            rollup_id: self.rollup_id,
        }
        .build();

        // spawn and save handle
        self.running_auctions.spawn(new_auction_id, auction.run());
        self.auction_handles.insert(new_auction_id, handle);
    }

    fn abort_auction(&mut self, auction_id: Id) -> eyre::Result<()> {
        let handle = self
            .auction_handles
            .get(&auction_id)
            .ok_or_eyre("unable to get handle for the given auction")?;
        handle.cancel();
        Ok(())
    }

    #[instrument(skip(self))]
    // pub(crate) fn start_timer(&mut self, auction_id: Id) -> eyre::Result<()> {
    pub(crate) fn start_timer(&mut self, block_commitment: Commitment) -> eyre::Result<()> {
        let auction_id =
            crate::auction::Id::from_sequencer_block_hash(block_commitment.sequencer_block_hash());

        if let Some(current_block) = &mut self.current_block {
            if current_block.commitment(block_commitment) {
                let handle = self
                    .auction_handles
                    .get_mut(&auction_id)
                    .ok_or_eyre("unable to get handle for the given auction")?;
                handle
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
                    "not starting the auction timer because the sequencer block hash of the \
                     commitment hash did not match
                    not match that of the currently running auction"
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
        let auction_id =
            crate::auction::Id::from_sequencer_block_hash(block.sequencer_block_hash());

        if let Some(current_block) = &mut self.current_block {
            if !current_block.execute(block) {
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
            } else {
                let handle = self
                    .auction_handles
                    .get_mut(&auction_id)
                    .ok_or_eyre("unable to get handle for the given auction")?;

                handle
                    .start_processing_bids()
                    .wrap_err("failed to send command to start processing bids")?;
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
        let auction_id =
            crate::auction::Id::from_sequencer_block_hash(bundle.base_sequencer_block_hash());
        if let Some(current_block) = &mut self.current_block {
            if let Err(e) = current_block
                .ensure_bundle_is_valid(&bundle)
                .wrap_err("failed to handle bundle")
            {
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
                return Err(e);
            } else {
                self.auction_handles
                    .get_mut(&auction_id)
                    .ok_or_eyre("unable to get handle for the given auction")?
                    .try_send_bundle(bundle)
                    .wrap_err("failed to add bundle to auction")?;
            }
        } else {
            info!(
                "cannot forward the received bundle to an auction because no auction is currently \
                 running; dropping the bundle"
            );
        }
        Ok(())
    }

    pub(crate) async fn join_next(&mut self) -> Option<(Id, eyre::Result<()>)> {
        if let Some((auction_id, result)) = self.running_auctions.join_next().await {
            // TODO: get rid of this expect somehow
            self.auction_handles
                .remove(&auction_id)
                .expect("handle should always exist for running auction");

            Some((auction_id, flatten_join_result(result)))
        } else {
            None
        }
    }

    pub(crate) fn abort_all(&mut self) -> usize {
        let number_of_live_auctions = self.running_auctions.len();
        self.running_auctions.abort_all();
        number_of_live_auctions
    }
}
