use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::{
    generated::composer::v1alpha1::{
        sequencer_hooks_service_server::SequencerHooksService,
        SendFinalizedHashRequest,
        SendFinalizedHashResponse,
        SendOptimisticBlockRequest,
        SendOptimisticBlockResponse,
    },
    protocol::transaction::v1alpha1::action::SequenceAction,
    Protobuf,
};
use astria_eyre::eyre::WrapErr;
use bytes::Bytes;
use pbjson_types::Timestamp;
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

pub(crate) struct OptimisticBlockInfo {
    block_hash: Bytes,
    seq_actions: Vec<SequenceAction>,
    time: Timestamp,
}

impl OptimisticBlockInfo {
    pub(crate) fn new(
        block_hash: Bytes,
        seq_actions: Vec<SequenceAction>,
        time: Timestamp,
    ) -> Self {
        Self {
            block_hash,
            seq_actions,
            time,
        }
    }

    pub(crate) fn block_hash(&self) -> Bytes {
        self.block_hash.clone()
    }

    pub(crate) fn seq_actions(&self) -> Vec<SequenceAction> {
        self.seq_actions.clone()
    }

    pub(crate) fn time(&self) -> Timestamp {
        self.time.clone()
    }
}

pub(crate) struct FinalizedHashInfo {
    block_hash: Bytes,
}

impl FinalizedHashInfo {
    pub(crate) fn new(block_hash: Bytes) -> Self {
        Self {
            block_hash,
        }
    }

    pub(crate) fn block_hash(&self) -> Bytes {
        self.block_hash.clone()
    }
}

pub(crate) struct SequencerHooks {
    optimistic_block_sender: mpsc::Sender<OptimisticBlockInfo>,
    finalized_hash_sender: mpsc::Sender<FinalizedHashInfo>,
}

impl SequencerHooks {
    pub(crate) fn new(
        optimistic_block_sender: mpsc::Sender<OptimisticBlockInfo>,
        finalized_hash_sender: mpsc::Sender<FinalizedHashInfo>,
    ) -> Self {
        Self {
            optimistic_block_sender,
            finalized_hash_sender,
        }
    }

    pub(crate) async fn send_optimistic_block_with_timeout(
        &self,
        req: OptimisticBlockInfo,
    ) -> Result<(), SendTimeoutError<OptimisticBlockInfo>> {
        self.optimistic_block_sender
            .send_timeout(req, Duration::from_secs(SEND_TIMEOUT))
            .await
    }

    pub(crate) async fn send_finalized_hash_with_timeout(
        &self,
        req: FinalizedHashInfo,
    ) -> Result<(), SendTimeoutError<FinalizedHashInfo>> {
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

        let mut seq_actions = vec![];
        for action in &inner.seq_action {
            match SequenceAction::try_from_raw_ref(action) {
                Ok(action) => seq_actions.push(action),
                Err(e) => {
                    info!("Failed to convert sequence action: {:?}", e);
                    return Err(Status::invalid_argument("invalid sequence action"));
                }
            }
        }

        return match self
            .send_optimistic_block_with_timeout(OptimisticBlockInfo::new(
                inner.block_hash,
                seq_actions,
                inner.time.unwrap(),
            ))
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
            .send_finalized_hash_with_timeout(FinalizedHashInfo::new(inner.block_hash))
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
