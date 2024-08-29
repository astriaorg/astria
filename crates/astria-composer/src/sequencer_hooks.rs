use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::generated::composer::v1alpha1::{
    sequencer_hooks_service_server::SequencerHooksService,
    SendFinalizedHashRequest,
    SendFinalizedHashResponse,
    SendOptimisticBlockRequest,
    SendOptimisticBlockResponse,
};
use astria_eyre::eyre::WrapErr;
use tokio::sync::{
    mpsc,
    mpsc::error::SendTimeoutError,
};
use tonic::{
    Request,
    Response,
    Status,
};
use tracing::info;

const SEND_TIMEOUT: u64 = 2;

// pub(crate) struct OptimisticBlockInfo {
//     block_hash: Bytes,
//     seq_actions: Vec<SequenceAction>,
//     time: Timestamp,
// }

pub(crate) struct SequencerHooks {
    filtered_block_sender: mpsc::Sender<SendOptimisticBlockRequest>,
    finalized_hash_sender: mpsc::Sender<SendFinalizedHashRequest>,
}

impl SequencerHooks {
    pub(crate) fn new(
        filtered_block_sender: mpsc::Sender<SendOptimisticBlockRequest>,
        finalized_hash_sender: mpsc::Sender<SendFinalizedHashRequest>,
    ) -> Self {
        Self {
            filtered_block_sender,
            finalized_hash_sender,
        }
    }

    pub(crate) async fn send_optimistic_block_with_timeout(
        &self,
        req: SendOptimisticBlockRequest,
    ) -> Result<(), SendTimeoutError<SendOptimisticBlockRequest>> {
        self.filtered_block_sender
            .send_timeout(req, Duration::from_secs(SEND_TIMEOUT))
            .await
    }

    pub(crate) async fn send_finalized_hash_with_timeout(
        &self,
        req: SendFinalizedHashRequest,
    ) -> Result<(), SendTimeoutError<SendFinalizedHashRequest>> {
        self.finalized_hash_sender
            .send_timeout(req, Duration::from_secs(SEND_TIMEOUT))
            .await
    }
}

#[async_trait::async_trait]
impl SequencerHooksService for SequencerHooks {
    async fn send_optimistic_block(
        self: Arc<Self>,
        request: Request<SendOptimisticBlockRequest>,
    ) -> Result<Response<SendOptimisticBlockResponse>, Status> {
        let inner = request.into_inner();
        return match self
            .send_optimistic_block_with_timeout(inner)
            .await
            .wrap_err("unable to send optimistic block to executor")
        {
            Ok(()) => Ok(Response::new(SendOptimisticBlockResponse {})),
            Err(e) => {
                info!("Failed to send optimistic block: {:?}", e);
                return Err(Status::internal("Failed to send optimistic block"));
            }
        };
    }

    async fn send_finalized_hash(
        self: Arc<Self>,
        request: Request<SendFinalizedHashRequest>,
    ) -> Result<Response<SendFinalizedHashResponse>, Status> {
        let inner = request.into_inner();

        return match self
            .send_finalized_hash_with_timeout(inner)
            .await
            .wrap_err("unable to send finalized block hash to executor")
        {
            Ok(()) => Ok(Response::new(SendFinalizedHashResponse {})),
            Err(e) => {
                info!("Failed to send finalized_block hash: {:?}", e);
                return Err(Status::internal("Failed to send finalized block hash"));
            }
        };
    }
}
