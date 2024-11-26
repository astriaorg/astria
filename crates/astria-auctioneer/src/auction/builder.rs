use std::time::Duration;

use astria_core::{
    generated::sequencerblock::v1::sequencer_service_client::SequencerServiceClient,
    primitive::v1::{
        asset,
        RollupId,
    },
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{
    Auction,
    Handle,
    Id,
    SequencerKey,
};
use crate::Metrics;

pub(crate) struct Builder {
    pub(crate) metrics: &'static Metrics,
    pub(crate) cancellation_token: CancellationToken,

    /// The endpoint for the sequencer gRPC service used to get pending nonces
    pub(crate) sequencer_grpc_client: SequencerServiceClient<tonic::transport::Channel>,
    /// The endpoint for the sequencer ABCI service used to submit transactions
    pub(crate) sequencer_abci_client: sequencer_client::HttpClient,
    /// The amount of time to wait after a commit before closing the auction for bids and
    /// submitting the resulting transaction
    pub(crate) latency_margin: Duration,
    /// The ID of the auction to be run
    pub(crate) auction_id: Id,
    /// The key used to sign sequencer transactions
    pub(crate) sequencer_key: SequencerKey,
    /// The denomination of the fee asset used in the sequencer transactions
    pub(crate) fee_asset_denomination: asset::Denom,
    /// The chain ID used for sequencer transactions
    pub(crate) sequencer_chain_id: String,
    /// The rollup ID used for `RollupDataSubmission` with the auction result
    pub(crate) rollup_id: RollupId,
}

impl Builder {
    pub(crate) fn build(self) -> (Handle, Auction) {
        let Self {
            metrics,
            cancellation_token,
            sequencer_grpc_client,
            sequencer_abci_client,
            latency_margin,
            auction_id,
            fee_asset_denomination,
            rollup_id,
            sequencer_key,
            sequencer_chain_id,
        } = self;

        // TODO: get the capacities from config or something instead of using a magic number
        let (commands_tx, commands_rx) = mpsc::channel(16);
        let (new_bundles_tx, new_bundles_rx) = mpsc::channel(16);

        let auction = Auction {
            metrics,
            cancellation_token: cancellation_token.clone(),
            sequencer_grpc_client,
            sequencer_abci_client,
            commands_rx,
            new_bundles_rx,
            latency_margin,
            id: auction_id,
            sequencer_key,
            fee_asset_denomination,
            sequencer_chain_id,
            rollup_id,
        };

        (
            Handle {
                cancellation_token,
                commands_tx,
                new_bundles_tx,
            },
            auction,
        )
    }
}
