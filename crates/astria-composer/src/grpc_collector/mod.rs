use std::{
    net::SocketAddr,
    time::Duration,
};

use astria_core::{
    generated::composer::v1alpha1::{
        grpc_collector_service_server::{
            GrpcCollectorService,
            GrpcCollectorServiceServer,
        },
        SubmitSequenceActionsRequest,
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
use tokio::{
    io,
    net::TcpListener,
    sync::mpsc::error::SendTimeoutError,
};
use tonic::{
    Request,
    Response,
};
use crate::executor;

pub(super) struct GrpcCollector {
    grpc_collector_listener: TcpListener,
}

impl GrpcCollector {
    pub(super) fn new(grpc_collector_listener: TcpListener) -> Self {
        Self {
            grpc_collector_listener,
        }
    }

    /// Returns the socker address the grpc collector is served over
    /// # Errors
    /// Returns an error if the listener is not bound
    pub(super) fn grpc_collector_local_addr(&self) -> io::Result<SocketAddr> {
        self.grpc_collector_listener.local_addr()
    }

    pub(super) async fn run_until_stopped(
        self,
        executor_handle: executor::Handle,
    ) -> eyre::Result<()> {
        let composer_service = GrpcCollectorServiceServer::new(executor_handle.clone());
        let grpc_server = tonic::transport::Server::builder().add_service(composer_service);

        match grpc_server
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(
                self.grpc_collector_listener,
            ))
            .await
        {
            Ok(()) => Ok(()),
            Err(err) => Err(eyre!(err)),
        }
    }
}

#[async_trait::async_trait]
impl GrpcCollectorService for executor::Handle {
    async fn submit_sequence_actions(
        &self,
        request: Request<SubmitSequenceActionsRequest>,
    ) -> Result<Response<()>, tonic::Status> {
        let submit_sequence_actions_request = request.into_inner();
        if submit_sequence_actions_request.sequence_actions.is_empty() {
            return Err(tonic::Status::invalid_argument(
                "No sequence actions provided",
            ));
        }

        // package the sequence actions into a SequenceAction and send it to the searcher
        for sequence_action in submit_sequence_actions_request.sequence_actions {
            let sequence_action = SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(sequence_action.rollup_id),
                data: sequence_action.tx_bytes,
                fee_asset_id: default_native_asset_id(),
            };

            match self
                .get()
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
