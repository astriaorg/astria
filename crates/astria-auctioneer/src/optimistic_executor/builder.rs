use std::time::Duration;

use astria_core::primitive::v1::RollupId;
use astria_eyre::eyre;
use tokio_util::sync::CancellationToken;

use super::OptimisticExecutor;
use crate::Metrics;

pub(crate) struct Builder {
    pub(crate) metrics: &'static Metrics,
    pub(crate) shutdown_token: CancellationToken,
    /// The endpoint for the sequencer gRPC service used for the optimistic block stream
    pub(crate) sequencer_grpc_endpoint: String,
    /// The endpoint for the sequencer ABCI service used to submit the auction winner transaction
    pub(crate) sequencer_abci_endpoint: String,
    /// The rollup ID for the filtered optimistic block stream
    pub(crate) rollup_id: String,
    /// The endpoint for the rollup's optimistic execution gRPC service
    pub(crate) optimistic_execution_grpc_endpoint: String,
    /// The endpoint for the rollup's bundle gRPC service
    pub(crate) bundle_grpc_endpoint: String,
    /// The amount of time to wait after a commit before closing the auction
    pub(crate) latency_margin: Duration,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<OptimisticExecutor> {
        let Self {
            metrics,
            shutdown_token,
            sequencer_grpc_endpoint,
            sequencer_abci_endpoint,
            rollup_id,
            optimistic_execution_grpc_endpoint,
            bundle_grpc_endpoint,
            latency_margin,
        } = self;

        let rollup_id = RollupId::from_unhashed_bytes(&rollup_id);

        Ok(OptimisticExecutor {
            metrics,
            shutdown_token,
            sequencer_grpc_endpoint,
            sequencer_abci_endpoint,
            rollup_id,
            rollup_grpc_endpoint: optimistic_execution_grpc_endpoint,
            bundle_grpc_endpoint,
            latency_margin,
        })
    }
}
