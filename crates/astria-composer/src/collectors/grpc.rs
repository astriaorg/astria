//! `GrpcCollector` implements the `GrpcCollectorService` rpc service.

use std::sync::Arc;

use astria_core::{
    generated::composer::v1alpha1::{
        grpc_collector_service_server::GrpcCollectorService,
        SubmitRollupTransactionRequest,
        SubmitRollupTransactionResponse,
    },
    primitive::v1::{
        asset::default_native_asset_id,
        RollupId,
    },
    protocol::transaction::v1alpha1::action::SequenceAction,
};
use tokio::sync::mpsc::error::SendTimeoutError;
use tonic::{
    Request,
    Response,
    Status,
};

use crate::{
    collectors::EXECUTOR_SEND_TIMEOUT,
    executor,
    metrics::Metrics,
};

/// Implements the `GrpcCollectorService` which listens for incoming gRPC requests and
/// sends the Rollup transactions to the Executor. The Executor then sends the transactions
/// to the Astria Shared Sequencer.
pub(crate) struct Grpc {
    executor: executor::Handle,
    metrics: &'static Metrics,
}

impl Grpc {
    pub(crate) fn new(executor: executor::Handle, metrics: &'static Metrics) -> Self {
        Self {
            executor,
            metrics,
        }
    }
}

#[async_trait::async_trait]
impl GrpcCollectorService for Grpc {
    async fn submit_rollup_transaction(
        self: Arc<Self>,
        request: Request<SubmitRollupTransactionRequest>,
    ) -> Result<Response<SubmitRollupTransactionResponse>, Status> {
        let submit_rollup_tx_request = request.into_inner();

        let Ok(rollup_id) = RollupId::try_from_slice(&submit_rollup_tx_request.rollup_id) else {
            return Err(Status::invalid_argument("invalid rollup id"));
        };

        let sequence_action = SequenceAction {
            rollup_id,
            data: submit_rollup_tx_request.data,
            fee_asset_id: default_native_asset_id(),
        };

        self.metrics.increment_grpc_txs_received(&rollup_id);
        match self
            .executor
            .send_timeout(sequence_action, EXECUTOR_SEND_TIMEOUT)
            .await
        {
            Ok(()) => {}
            Err(SendTimeoutError::Timeout(_seq_action)) => {
                self.metrics.increment_grpc_txs_dropped(&rollup_id);
                return Err(Status::unavailable("timeout while sending txs to composer"));
            }
            Err(SendTimeoutError::Closed(_seq_action)) => {
                self.metrics.increment_grpc_txs_dropped(&rollup_id);
                return Err(Status::failed_precondition("composer is not available"));
            }
        }

        Ok(Response::new(SubmitRollupTransactionResponse {}))
    }
}
