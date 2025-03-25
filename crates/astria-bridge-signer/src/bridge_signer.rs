use std::time::Duration;

use astria_core::generated::astria::signer::v1::frost_participant_service_server::FrostParticipantServiceServer;
use astria_eyre::eyre::{
    self,
    eyre,
    Report,
    WrapErr as _,
};
use futures::TryFutureExt as _;
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
    sync::{
        oneshot,
        watch,
    },
    task::JoinHandle,
    time::timeout,
};
use tracing::{
    error,
    info,
    instrument,
};

use crate::{
    Config,
    Metrics,
    Server,
};

const GRPC_SERVER_SHUTDOWN_DURATION: Duration = Duration::from_secs(5);

struct GrpcServerHandle {
    handle: Option<JoinHandle<Result<(), tonic::transport::Error>>>,
    shutdown_tx: oneshot::Sender<()>,
}

/// [`BridgeSigner`] is a threshold signer node responsible for partially signing
/// [sequencer transactions](astria_core::protocol::transaction::v1::Transaction)
/// produced by the Astria Bridge Withdrawer, which then collects the partials
/// to form a complete signature for the transaction.
pub struct BridgeSigner {
    grpc_server_handle: GrpcServerHandle,
    signal_receiver: SignalReceiver,
}

impl BridgeSigner {
    #[instrument(skip_all, err)]
    pub fn from_config(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let server = Server::new(
            cfg.frost_secret_key_package_path,
            cfg.rollup_rpc_endpoint,
            metrics,
        )
        .wrap_err("failed initializing bridge signer gRPC server")?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let grpc_server = tonic::transport::Server::builder()
            .add_service(FrostParticipantServiceServer::new(server));

        let grpc_addr: std::net::SocketAddr = cfg
            .grpc_endpoint
            .parse()
            .wrap_err("failed to parse grpc endpoint")?;

        info!(grpc_addr = grpc_addr.to_string(), "starting grpc server");
        let grpc_server_handle = tokio::task::spawn(
            grpc_server.serve_with_shutdown(grpc_addr, shutdown_rx.unwrap_or_else(|_| ())),
        );

        let signal_receiver = spawn_signal_handler();

        Ok(Self {
            grpc_server_handle: GrpcServerHandle {
                handle: Some(grpc_server_handle),
                shutdown_tx,
            },
            signal_receiver,
        })
    }

    pub async fn run_until_stopped(mut self) -> eyre::Result<()> {
        tokio::select!(
            res = self.grpc_server_handle.handle.as_mut().expect("gRPC server handle must be set at this point") => {
                info!("gRPC server exited without receiving shutdown signal. This should not happen");
                self.grpc_server_handle.handle.take();
                match res {
                    Ok(Ok(())) => Err(eyre!("gRPC server exited unexpectedly")),
                    Ok(Err(err)) => Err(Report::new(err).wrap_err("gRPC server exited with error")),
                    Err(err) => Err(Report::new(err).wrap_err("executor panicked")),
                }
            }
            _ = self.signal_receiver.stop_rx.changed() => {
                info!("received shutdown signal, shutting down");
                self.shutdown().await;
                Ok(())
            }
        )
    }

    #[instrument(skip_all)]
    async fn shutdown(self) {
        let _ = self.grpc_server_handle.shutdown_tx.send(());

        if let Some(handle) = self.grpc_server_handle.handle {
            match timeout(GRPC_SERVER_SHUTDOWN_DURATION, handle).await {
                Ok(Ok(_)) => info!("gRPC server stopped"),
                Ok(Err(e)) => error!(%e, "gRPC server failed"),
                Err(_) => error!("gRPC server failed to shut down in time"),
            }
        } else {
            info!("shutdown called but gRPC server handle is not present. This should not happen")
        }
    }
}

struct SignalReceiver {
    stop_rx: watch::Receiver<()>,
}

fn spawn_signal_handler() -> SignalReceiver {
    let (stop_tx, stop_rx) = watch::channel(());
    tokio::spawn(async move {
        let mut sigint = signal(SignalKind::interrupt()).expect(
            "setting a SIGINT listener should always work on unix; is this running on unix?",
        );
        let mut sigterm = signal(SignalKind::terminate()).expect(
            "setting a SIGTERM listener should always work on unix; is this running on unix?",
        );
        loop {
            select! {
                _ = sigint.recv() => {
                    info!("received SIGINT");
                    let _ = stop_tx.send(());
                    break;
                }
                _ = sigterm.recv() => {
                    info!("received SIGTERM");
                    let _ = stop_tx.send(());
                    break;
                }
            }
        }
    });

    SignalReceiver {
        stop_rx,
    }
}
