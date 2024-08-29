use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::generated::{
    composer::v1alpha1::{
        sequencer_hooks_service_server::SequencerHooksService,
        SendFinalizedHashRequest,
        SendFinalizedHashResponse,
        SendOptimisticBlockResponse,
    },
    sequencerblock::v1alpha1::FilteredSequencerBlock,
};
use bytes::Bytes;
use tokio::sync::{
    mpsc,
    mpsc::error::SendTimeoutError,
};
use tonic::{
    Request,
    Response,
    Status,
};

const SEND_TIMEOUT: u64 = 2;

pub(crate) struct SequencerHooks {
    filtered_block_sender: mpsc::Sender<FilteredSequencerBlock>,
    finalized_hash_sender: mpsc::Sender<Bytes>,
}

impl SequencerHooks {
    pub(crate) fn new(
        filtered_block_sender: mpsc::Sender<FilteredSequencerBlock>,
        finalized_hash_sender: mpsc::Sender<Bytes>,
    ) -> Self {
        Self {
            filtered_block_sender,
            finalized_hash_sender,
        }
    }

    pub(crate) async fn send_filtered_block_with_timeout(
        &self,
        block: FilteredSequencerBlock,
    ) -> Result<(), SendTimeoutError<FilteredSequencerBlock>> {
        self.filtered_block_sender
            .send_timeout(block, Duration::from_secs(SEND_TIMEOUT))
            .await
    }

    pub(crate) async fn send_finalized_hash_with_timeout(
        &self,
        hash: Bytes,
    ) -> Result<(), SendTimeoutError<Bytes>> {
        self.finalized_hash_sender
            .send_timeout(hash, Duration::from_secs(SEND_TIMEOUT))
            .await
    }
}

#[async_trait::async_trait]
impl SequencerHooksService for SequencerHooks {
    async fn send_optimistic_block(
        self: Arc<Self>,
        request: Request<FilteredSequencerBlock>,
    ) -> Result<Response<SendOptimisticBlockResponse>, Status> {
        let block = request.into_inner();
        match self.send_filtered_block_with_timeout(block).await {
            Ok(_) => Ok(Response::new(SendOptimisticBlockResponse {})),
            Err(SendTimeoutError::Timeout(block)) => Err(Status::deadline_exceeded("failed to send optimistic block")),
            Err(SendTimeoutError::Closed(block)) => Err(Status::cancelled("failed to send optimistic block")),
        }
    }

    async fn send_finalized_hash(
        self: Arc<Self>,
        request: Request<SendFinalizedHashRequest>,
    ) -> Result<Response<SendFinalizedHashResponse>, Status> {
        let hash = request.into_inner().block_hash;
        match self.send_finalized_hash_with_timeout(hash).await {
            Ok(_) => Ok(Response::new(SendFinalizedHashResponse {})),
            Err(SendTimeoutError::Timeout(hash)) => Err(Status::deadline_exceeded("failed to send finalized hash")),
            Err(SendTimeoutError::Closed(hash)) => Err(Status::cancelled("failed to send finalized hash")),
        }
    }
}
