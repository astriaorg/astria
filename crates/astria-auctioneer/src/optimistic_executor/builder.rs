use astria_eyre::eyre;
use tokio::sync::{
    mpsc,
    watch,
};

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
    pub(crate) fn build(self) -> eyre::Result<OptimisticExecutor> {
        let Self {
            metrics,
            sequencer_grpc_endpoint,
        } = self;

        let (_executed_blocks_tx, executed_blocks_rx) = mpsc::channel(16);
        let (_optimistic_blocks_tx, optimistic_blocks_rx) = mpsc::channel(16);
        let (_block_commitments_tx, block_commitments_rx) = mpsc::channel(17);

        // TODO: replace with grpc streams

        Ok(OptimisticExecutor {
            optimistic_blocks_rx,
            executed_blocks_rx,
            block_commitments_rx,
            block: todo!("replace with block_tx or somethingg?"),
        })
    }
}
