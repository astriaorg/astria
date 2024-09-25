use std::time::Duration;

use astria_eyre::eyre;
use tokio::sync::{
    mpsc,
    oneshot,
};
use tokio_util::sync::CancellationToken;

use super::{
    BundlesHandle,
    Driver,
    Id,
    OptimisticExecutionHandle,
};
use crate::Metrics;

pub(crate) struct Builder {
    pub(crate) metrics: &'static Metrics,
    pub(crate) shutdown_token: CancellationToken,

    /// The endpoint for the sequencer gRPC service used to get pending nonces
    pub(crate) sequencer_grpc_endpoint: String,
    /// The endpoint for the sequencer ABCI service used to submit transactions
    pub(crate) sequencer_abci_endpoint: String,
    /// The amount of time to wait after a commit before closing the auction for bids and
    /// submitting the resulting transaction
    pub(crate) latency_margin: Duration,
    /// The ID of the auction to be run
    pub(crate) auction_id: Id,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<(Driver, OptimisticExecutionHandle, BundlesHandle)> {
        let Self {
            metrics,
            shutdown_token,
            sequencer_grpc_endpoint,
            sequencer_abci_endpoint,
            latency_margin,
            auction_id,
        } = self;

        let (executed_block_tx, executed_block_rx) = oneshot::channel();
        let (block_commitment_tx, block_commitment_rx) = oneshot::channel();
        let (reorg_tx, reorg_rx) = oneshot::channel();
        // TODO: get the capacity from config or something instead of using a magic number
        let (new_bids_tx, new_bids_rx) = mpsc::channel(16);

        let driver = Driver {
            metrics,
            shutdown_token,
            sequencer_grpc_endpoint,
            sequencer_abci_endpoint,
            executed_block_rx,
            block_commitment_rx,
            reorg_rx,
            new_bids_rx,
            auction_id,
            latency_margin,
        };

        Ok((
            driver,
            OptimisticExecutionHandle {
                executed_block_tx: Some(executed_block_tx),
                block_commitment_tx: Some(block_commitment_tx),
                reorg_tx: Some(reorg_tx),
            },
            BundlesHandle {
                new_bids_tx,
            },
        ))
    }
}
