/// The auction Manager is responsible for managing running auction futures and their
/// associated handles.
use astria_core::{
    primitive::v1::{
        asset,
        RollupId,
    },
    sequencerblock::v1::block::FilteredSequencerBlock,
};
use tokio::sync::mpsc;

use super::{
    Auction,
    PendingNonceSubscriber,
    SequencerKey,
    Worker,
};

pub(in crate::auctioneer::inner) struct Factory {
    #[allow(dead_code)]
    pub(in crate::auctioneer::inner) metrics: &'static crate::Metrics,
    pub(in crate::auctioneer::inner) sequencer_abci_client: sequencer_client::HttpClient,
    pub(in crate::auctioneer::inner) latency_margin: std::time::Duration,
    pub(in crate::auctioneer::inner) sequencer_key: SequencerKey,
    pub(in crate::auctioneer::inner) fee_asset_denomination: asset::Denom,
    pub(in crate::auctioneer::inner) sequencer_chain_id: String,
    pub(in crate::auctioneer::inner) rollup_id: RollupId,
    pub(in crate::auctioneer::inner) pending_nonce: PendingNonceSubscriber,
}

impl Factory {
    pub(in crate::auctioneer::inner) fn start_new(
        &self,
        block: &FilteredSequencerBlock,
    ) -> Auction {
        let auction_id = super::Id::from_sequencer_block_hash(block.block_hash());
        let height = block.height().into();

        // TODO: get the capacities from config or something instead of using a magic number
        let (commands_tx, commands_rx) = mpsc::channel(16);
        let (bundles_tx, bundles_rx) = mpsc::channel(16);

        let auction = Worker {
            sequencer_abci_client: self.sequencer_abci_client.clone(),
            commands_rx,
            bundles_rx,
            latency_margin: self.latency_margin,
            id: auction_id,
            sequencer_key: self.sequencer_key.clone(),
            fee_asset_denomination: self.fee_asset_denomination.clone(),
            sequencer_chain_id: self.sequencer_chain_id.clone(),
            rollup_id: self.rollup_id,
            pending_nonce: self.pending_nonce.clone(),
        };

        Auction {
            id: auction_id,
            height,
            parent_block_of_executed: None,
            commands: commands_tx,
            bundles: bundles_tx,
            worker: tokio::task::spawn(auction.run()),
        }
    }
}
