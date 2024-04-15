use std::net::SocketAddr;

use astria_eyre::{
    eyre,
    eyre::Context,
};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::{
    collectors,
    executor,
    grpc::GrpcServer,
};

pub(crate) struct Builder {
    pub(crate) grpc_addr: SocketAddr,
    pub(crate) executor: executor::Handle,
    pub(crate) shutdown_token: CancellationToken,
}

impl Builder {
    pub(crate) async fn build(self) -> eyre::Result<GrpcServer> {
        let Self {
            grpc_addr,
            executor,
            shutdown_token,
        } = self;

        let listener = TcpListener::bind(grpc_addr)
            .await
            .wrap_err("failed to bind grpc listener")?;
        let grpc_collector = collectors::Grpc::new(executor.clone());

        Ok(GrpcServer {
            listener,
            grpc_collector,
            shutdown_token,
        })
    }
}
