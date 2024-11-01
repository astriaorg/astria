use astria_core::primitive::v1::RollupId;
use tokio_util::sync::CancellationToken;

use super::Startup;
use crate::{
    auction,
    Metrics,
};

pub(crate) struct Builder {
    pub(crate) metrics: &'static Metrics,
    pub(crate) shutdown_token: CancellationToken,
    /// The endpoint for the sequencer gRPC service used for the optimistic block stream
    pub(crate) sequencer_grpc_endpoint: String,
    /// The file path for the private key used to sign sequencer transactions with the auction
    /// results
    /// The rollup ID for the filtered optimistic block stream
    pub(crate) rollup_id: String,
    /// The endpoint for the rollup's optimistic execution gRPC service
    pub(crate) rollup_grpc_endpoint: String,
    /// Manager for ongoing auctions
    pub(crate) auctions: auction::Manager,
}

impl Builder {
    pub(crate) fn build(self) -> Startup {
        let Self {
            metrics,
            shutdown_token,
            sequencer_grpc_endpoint,
            rollup_id,
            rollup_grpc_endpoint,
            auctions,
        } = self;

        let rollup_id = RollupId::from_unhashed_bytes(&rollup_id);

        Startup {
            metrics,
            shutdown_token,
            sequencer_grpc_endpoint,
            rollup_id,
            rollup_grpc_endpoint,
            auctions,
        }
    }
}
