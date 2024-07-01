//! `GrpcCollector` implements the `GrpcCollectorService` rpc service.

use std::sync::Arc;

use astria_core::{
    generated::composer::v1alpha1::{
        grpc_collector_service_server::GrpcCollectorService,
        SubmitRollupTransactionRequest,
        SubmitRollupTransactionResponse,
    },
    primitive::v1::{
        asset,
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
use astria_core::protocol::transaction::v1alpha1::Action;

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
    fee_asset: asset::Denom,
}

impl Grpc {
    pub(crate) fn new(
        executor: executor::Handle,
        metrics: &'static Metrics,
        fee_asset: asset::Denom,
    ) -> Self {
        Self {
            executor,
            metrics,
            fee_asset,
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

        let action = if let Some(action) = submit_rollup_tx_request.action {
            action
        } else {
            return Err(Status::invalid_argument("missing action"));
        };

        let new_action = if let Ok(action) = Action::try_from_raw(action) {
            action
        } else {
            return Err(Status::invalid_argument("invalid action"));
        };

        match self
            .executor
            .send_timeout(new_action, EXECUTOR_SEND_TIMEOUT)
            .await
        {
            Ok(()) => {}
            Err(SendTimeoutError::Timeout(_seq_action)) => {
                return Err(Status::unavailable("timeout while sending txs to composer"));
            }
            Err(SendTimeoutError::Closed(_seq_action)) => {
                return Err(Status::failed_precondition("composer is not available"));
            }
        }

        Ok(Response::new(SubmitRollupTransactionResponse {}))
    }
}
