// ! `GrpcCollector` allows users to directly send Rollup transactions to the Composer
//
// ! The [`GrpcCollector`] listens for incoming gRPC requests and sends the Rollup
// transactions to the ! Executor. The Executor then sends the transactions to the Astria
// Shared Sequencer.

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
    eyre::WrapErr,
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

use crate::{
    executor,
    executor::Handle,
};

/// `GrpcCollector` listens for incoming gRPC requests and sends the Rollup transactions to the
/// Executor. The Executor then sends the transactions to the Astria Shared Sequencer.
///
/// It implements the `ComposerService` rpc service.
///
/// The composer will only have one `GrpcCollector` running at a time.
pub(crate) struct Grpc {
    listener: TcpListener,
    executor_handle: executor::Handle,
}

impl Grpc {
    pub(crate) async fn new(
        grpc_addr: SocketAddr,
        executor_handle: executor::Handle,
    ) -> eyre::Result<Self> {
        let listener = TcpListener::bind(grpc_addr)
            .await
            .wrap_err("failed to bind grpc listener")?;

        Ok(Self {
            listener,
            executor_handle,
        })
    }

    /// Returns the socket address the grpc collector is served over
    /// # Errors
    /// Returns an error if the listener is not bound
    pub(crate) fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub(crate) async fn run_until_stopped(self) -> eyre::Result<()> {
        let (mut health_reporter, health_service) = tonic_health::server::health_reporter();

        let composer_service = GrpcCollectorServiceServer::new(self.executor_handle);
        let grpc_server = tonic::transport::Server::builder()
            .add_service(health_service)
            .add_service(composer_service);

        health_reporter
            .set_serving::<GrpcCollectorServiceServer<Handle>>()
            .await;

        grpc_server
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(
                self.listener,
            ))
            .await
            .wrap_err("failed to run grpc server")
    }
}

#[async_trait::async_trait]
impl GrpcCollectorService for executor::Handle {
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

        // package the rollup txs into a SequenceAction and send it to the searcher
        for rollup_txs in submit_rollup_txs_request.rollup_txs {
            let sequence_action = SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(rollup_txs.rollup_id),
                data: rollup_txs.tx_bytes,
                fee_asset_id: default_native_asset_id(),
            };

            match self
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
