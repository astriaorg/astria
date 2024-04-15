//! `GrpcServer` allows users to directly send Rollup transactions to the Composer
//!
//! The [`GrpcServer`] listens for incoming gRPC requests and sends the Rollup
//! transactions to the Executor. The Executor then sends the transactions to the Astria
//! Shared Sequencer.
//!
//! It also implements the tonic health service.

use std::net::SocketAddr;

use astria_core::generated::composer::v1alpha1::grpc_collector_service_server::GrpcCollectorServiceServer;
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use tokio::{
    io,
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;

use crate::collectors;

/// Listens for incoming gRPC requests and sends the Rollup transactions to the
/// Executor. The Executor then sends the transactions to the Astria Shared Sequencer.
///
/// It implements the `GrpcCollectorServiceServer` rpc service and also the tonic health service
pub(crate) struct GrpcServer {
    pub(super) listener: TcpListener,
    pub(super) grpc_collector: collectors::Grpc,
    pub(super) shutdown_token: CancellationToken,
}

impl GrpcServer {
    /// Returns the socket address the grpc server is served over
    /// # Errors
    /// Returns an error if the listener is not bound
    pub(crate) fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub(crate) async fn run_until_stopped(self) -> eyre::Result<()> {
        let (mut health_reporter, health_service) = tonic_health::server::health_reporter();

        let composer_service = GrpcCollectorServiceServer::new(self.grpc_collector);
        let grpc_server = tonic::transport::Server::builder()
            .add_service(health_service)
            .add_service(composer_service);

        health_reporter
            .set_serving::<GrpcCollectorServiceServer<collectors::Grpc>>()
            .await;

        grpc_server
            .serve_with_incoming_shutdown(
                tokio_stream::wrappers::TcpListenerStream::new(self.listener),
                self.shutdown_token.cancelled(),
            )
            .await
            .wrap_err("failed to run grpc server")
    }
}
