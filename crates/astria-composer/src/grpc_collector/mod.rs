use std::time::Duration;

use astria_core::{
    generated::composer::v1alpha1::{
        grpc_collector_service_server::{
            GrpcCollectorService,
            GrpcCollectorServiceServer,
        },
        SubmitRollupTxsRequest,
    },
    sequencer::v1::{
        asset::default_native_asset_id,
        transaction::action::SequenceAction,
        RollupId,
    },
};
use astria_eyre::{
    eyre,
    eyre::eyre,
};
use futures::TryFutureExt;
use tokio::{
    net::TcpListener,
    sync::mpsc::error::SendTimeoutError,
};
use tonic::{
    Request,
    Response,
};

use crate::executor::ExecutorHandle;

pub(super) struct GrpcCollector {
    grpc_collector_listener: TcpListener,
    executor_handle: ExecutorHandle,
    shutdown_channel: tokio::sync::oneshot::Receiver<()>,
}

impl GrpcCollector {
    pub(super) fn new(
        grpc_collector_listener: TcpListener,
        executor_handle: ExecutorHandle,
        shutdown_channel: tokio::sync::oneshot::Receiver<()>,
    ) -> Self {
        Self {
            grpc_collector_listener,
            executor_handle,
            shutdown_channel,
        }
    }

    pub(super) async fn run_until_stopped(self) -> eyre::Result<()> {
        let composer_service = GrpcCollectorServiceServer::new(self.executor_handle);
        let grpc_server = tonic::transport::Server::builder().add_service(composer_service);

        match grpc_server
            .serve_with_incoming_shutdown(
                tokio_stream::wrappers::TcpListenerStream::new(self.grpc_collector_listener),
                self.shutdown_channel.unwrap_or_else(|_| ()),
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(err) => Err(eyre!(err)),
        }
    }
}

#[async_trait::async_trait]
impl GrpcCollectorService for ExecutorHandle {
    async fn submit_rollup_txs(
        &self,
        request: Request<SubmitRollupTxsRequest>,
    ) -> Result<Response<()>, tonic::Status> {
        let submit_rollup_txs_request = request.into_inner();
        if submit_rollup_txs_request.rollup_txs.is_empty() {
            return Err(tonic::Status::invalid_argument(
                "No sequence actions provided",
            ));
        }

        // package the sequence actions into a SequenceAction and send it to the searcher
        for sequence_action in submit_rollup_txs_request.rollup_txs {
            let sequence_action = SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(sequence_action.rollup_id),
                data: sequence_action.tx_bytes,
                fee_asset_id: default_native_asset_id(),
            };

            match self
                .sequence_action_tx
                .send_timeout(sequence_action, Duration::from_millis(500))
                .await
            {
                Ok(()) => {}
                Err(SendTimeoutError::Timeout(_seq_action)) => {
                    return Err(tonic::Status::deadline_exceeded(
                        "timeout while sending txs to searcher",
                    ));
                }
                Err(SendTimeoutError::Closed(_seq_action)) => {
                    return Err(tonic::Status::unavailable("searcher is not available"));
                }
            }
        }

        Ok(Response::new(()))
    }
}
