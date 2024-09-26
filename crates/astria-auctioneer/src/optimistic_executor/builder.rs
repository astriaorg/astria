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

        let (executed_blocks_tx, executed_blocks_rx) = mpsc::channel(16);
        let (committed_blocks_tx, committed_blocks_rx) = mpsc::channel(16);

        let (block_rx, block_tx) = watch::channel(None);

        Ok(OptimisticExecutor {
            optimistic_blocks_rx: todo!(),
            executed_blocks_rx: todo!(),
            block_commitments_rx: todo!(),
            block: todo!("replace with block_tx or somethingg?"),
        })
    }
}
