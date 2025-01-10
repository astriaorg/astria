/// The auction Manager is responsible for managing running auction futures and their
/// associated handles.
use astria_core::{
    primitive::v1::{
        asset,
        RollupId,
    },
    sequencerblock::v1::block::FilteredSequencerBlock,
};
use tokio::sync::{
    mpsc,
    oneshot,
};
use tokio_util::sync::CancellationToken;

use super::{
    Auction,
    SequencerKey,
    Worker,
};
use crate::sequencer_channel::SequencerChannel;

pub(in crate::auctioneer::inner) struct Factory {
    #[allow(dead_code)]
    pub(in crate::auctioneer::inner) metrics: &'static crate::Metrics,
    pub(in crate::auctioneer::inner) sequencer_abci_client: sequencer_client::HttpClient,
    pub(in crate::auctioneer::inner) sequencer_channel: SequencerChannel,
    pub(in crate::auctioneer::inner) latency_margin: std::time::Duration,
    pub(in crate::auctioneer::inner) sequencer_key: SequencerKey,
    pub(in crate::auctioneer::inner) fee_asset_denomination: asset::Denom,
    pub(in crate::auctioneer::inner) sequencer_chain_id: String,
    pub(in crate::auctioneer::inner) rollup_id: RollupId,
    pub(in crate::auctioneer::inner) cancellation_token: CancellationToken,
    /// `last_successful_nonce + 1` is used for submitting an auction winner to Sequencer
    /// if an auction worker was not able to receive the last pending
    /// nonce from Sequencer in time. Starts unset at the beginning of the program and
    /// is set externally via Factory::set_last_succesful_nonce`.
    pub(in crate::auctioneer::inner) last_successful_nonce: Option<u32>,
}

impl Factory {
    pub(in crate::auctioneer::inner) fn start_new(
        &self,
        block: &FilteredSequencerBlock,
    ) -> Auction {
        let id = super::Id::from_sequencer_block_hash(block.block_hash());
        let block_hash = *block.block_hash();
        let height = block.height().into();

        // TODO: get the capacities from config or something instead of using a magic number
        let (start_bids_tx, start_bids_rx) = oneshot::channel();
        let (start_timer_tx, start_timer_rx) = oneshot::channel();
        let (bids_tx, bids_rx) = mpsc::unbounded_channel();

        let cancellation_token = self.cancellation_token.child_token();
        let auction = Worker {
            sequencer_abci_client: self.sequencer_abci_client.clone(),
            sequencer_channel: self.sequencer_channel.clone(),
            start_bids: Some(start_bids_rx),
            start_timer: Some(start_timer_rx),
            bids: bids_rx,
            latency_margin: self.latency_margin,
            id,
            sequencer_key: self.sequencer_key.clone(),
            fee_asset_denomination: self.fee_asset_denomination.clone(),
            sequencer_chain_id: self.sequencer_chain_id.clone(),
            rollup_id: self.rollup_id,
            cancellation_token: cancellation_token.clone(),
            last_successful_nonce: self.last_successful_nonce,
        };

        Auction {
            id,
            block_hash,
            height,
            hash_of_executed_block_on_rollup: None,
            start_bids: Some(start_bids_tx),
            start_timer: Some(start_timer_tx),
            bids: bids_tx,
            cancellation_token,
            worker: tokio::task::spawn(auction.run()),
        }
    }

    pub(in crate::auctioneer::inner) fn set_last_successful_nonce(&mut self, nonce: u32) {
        self.last_successful_nonce.replace(nonce);
    }
}
