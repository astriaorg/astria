use std::time::Duration;

use astria_core::primitive::v1::{
    asset,
    RollupId,
};
use tokio::sync::mpsc;

use super::{
    Auction,
    Handle,
    Id,
    PendingNonceSubscriber,
    SequencerKey,
};
pub(in crate::auctioneer) struct Builder {
    /// The endpoint for the sequencer ABCI service used to submit transactions
    pub(super) sequencer_abci_client: sequencer_client::HttpClient,
    /// The amount of time to wait after a commit before closing the auction for bids and
    /// submitting the resulting transaction
    pub(super) latency_margin: Duration,
    /// The ID of the auction to be run
    pub(super) auction_id: Id,
    /// The key used to sign sequencer transactions
    pub(super) sequencer_key: SequencerKey,
    /// The denomination of the fee asset used in the sequencer transactions
    pub(super) fee_asset_denomination: asset::Denom,
    /// The chain ID used for sequencer transactions
    pub(super) sequencer_chain_id: String,
    /// The rollup ID used for `RollupDataSubmission` with the auction result
    pub(super) rollup_id: RollupId,
    pub(super) pending_nonce: PendingNonceSubscriber,
}

impl Builder {
    pub(super) fn build(self) -> (Handle, Auction) {
        let Self {
            sequencer_abci_client,
            latency_margin,
            auction_id,
            fee_asset_denomination,
            rollup_id,
            sequencer_key,
            sequencer_chain_id,
            pending_nonce,
        } = self;

        // TODO: get the capacities from config or something instead of using a magic number
        let (commands_tx, commands_rx) = mpsc::channel(16);
        let (new_bundles_tx, new_bundles_rx) = mpsc::channel(16);

        let auction = Auction {
            sequencer_abci_client,
            commands_rx,
            new_bundles_rx,
            latency_margin,
            id: auction_id,
            sequencer_key,
            fee_asset_denomination,
            sequencer_chain_id,
            rollup_id,
            pending_nonce,
        };

        (
            Handle {
                commands_tx,
                new_bundles_tx,
            },
            auction,
        )
    }
}
