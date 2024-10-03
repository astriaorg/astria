use astria_core::primitive::v1::RollupId;
use astria_eyre::eyre;
use tokio::sync::{
    mpsc,
    watch,
};
use tokio_util::sync::CancellationToken;

use super::{
    Handle,
    OptimisticExecutor,
};
use crate::Metrics;

pub(crate) struct Builder {
    pub(crate) metrics: &'static Metrics,
    pub(crate) shutdown_token: CancellationToken,
    /// The endpoint for the sequencer gRPC service used for the optimistic block stream
    pub(crate) sequencer_grpc_endpoint: String,
    /// The rollup ID for the filtered optimistic block stream
    pub(crate) rollup_id: String,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<OptimisticExecutor> {
        let Self {
            metrics,
            shutdown_token,
            sequencer_grpc_endpoint,
            rollup_id,
        } = self;

        let rollup_id = RollupId::from_unhashed_bytes(&rollup_id);

        Ok(OptimisticExecutor {
            metrics,
            shutdown_token,
            sequencer_grpc_endpoint,
            rollup_id,
        })
    }
}
