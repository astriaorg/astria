//! `GrpcServer` allows users to directly send Rollup transactions to the Composer
//!
//! The [`GrpcServer`] listens for incoming gRPC requests and sends the Rollup
//! transactions to the Executor. The Executor then sends the transactions to the Astria
//! Shared Sequencer.
//!
//! It also implements the tonic health service.

use std::net::SocketAddr;

use astria_core::{
    generated::composer::v1alpha1::{
        grpc_collector_service_server::GrpcCollectorServiceServer,
        sequencer_hooks_service_server::SequencerHooksServiceServer,
    },
    primitive::v1::asset,
};
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use tokio::{
    io,
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    info,
    instrument,
};

use crate::{
    collectors,
    executor,
    metrics::Metrics,
    sequencer_hooks::SequencerHooks,
};

/// Listens for incoming gRPC requests and sends the Rollup transactions to the
/// Executor. The Executor then sends the transactions to the Astria Shared Sequencer.
///
/// It implements the `GrpcCollectorServiceServer` rpc service and also the tonic health service
pub(crate) struct GrpcServer {
    listener: TcpListener,
    grpc_collector: collectors::Grpc,
    sequencer_hooks: SequencerHooks,
    shutdown_token: CancellationToken,
}

pub(crate) struct Builder {
    pub(crate) grpc_addr: SocketAddr,
    pub(crate) executor: executor::Handle,
    pub(crate) shutdown_token: CancellationToken,
    pub(crate) metrics: &'static Metrics,
    pub(crate) fee_asset: asset::Denom,
    pub(crate) sequencer_hooks: SequencerHooks,
}

impl Builder {
    #[instrument(skip_all, err)]
    pub(crate) async fn build(self) -> eyre::Result<GrpcServer> {
        let Self {
            grpc_addr,
            executor,
            shutdown_token,
            metrics,
            fee_asset,
            sequencer_hooks,
        } = self;

        let listener = TcpListener::bind(grpc_addr)
            .await
            .wrap_err("failed to bind socket address")?;
        let grpc_collector = collectors::Grpc::new(executor.clone(), metrics, fee_asset);

        Ok(GrpcServer {
            listener,
            grpc_collector,
            sequencer_hooks,
            shutdown_token,
        })
    }
}

impl GrpcServer {
    /// Returns the socket address the grpc server is served over
    /// # Errors
    /// Returns an error if the listener is not bound
    pub(crate) fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub(crate) async fn run_until_stopped(self) -> eyre::Result<()> {
        info!("launching grpc server with grpc collector and sequencer hooks!");
        let (mut health_reporter, health_service) = tonic_health::server::health_reporter();

        let composer_service = GrpcCollectorServiceServer::new(self.grpc_collector);
        let sequencer_hooks_service = SequencerHooksServiceServer::new(self.sequencer_hooks);
        let grpc_server = tonic::transport::Server::builder()
            .add_service(health_service)
            .add_service(composer_service)
            .add_service(sequencer_hooks_service);

        health_reporter
            .set_serving::<GrpcCollectorServiceServer<collectors::Grpc>>()
            .await;
        health_reporter
            .set_serving::<SequencerHooksServiceServer<SequencerHooks>>()
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
