use astria_eyre::eyre;
use tokio::sync::mpsc;

use super::{
    Handle,
    OptimisticExecutor,
};
use crate::Metrics;

pub(crate) struct Builder {
    pub(crate) metrics: &'static Metrics,
    /// The endpoint for the sequencer gRPC service used for the optimistic block stream
    pub(crate) sequencer_grpc_endpoint: String,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<(OptimisticExecutor, Handle)> {
        let Self {
            metrics,
            sequencer_grpc_endpoint,
        } = self;

        let (executed_blocks_tx, executed_blocks_rx) = mpsc::channel(16);
        let (committed_blocks_tx, committed_blocks_rx) = mpsc::channel(16);

        Ok((
            OptimisticExecutor {
                metrics,
                sequencer_grpc_endpoint,
                executed_blocks_tx,
                committed_blocks_tx,
            },
            Handle {
                executed_blocks_rx,
                committed_blocks_rx,
            },
        ))
    }
}
