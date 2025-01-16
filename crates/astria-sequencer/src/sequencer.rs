use std::{
    fmt::Display,
    net::SocketAddr,
    path::PathBuf,
    str::FromStr,
};

use astria_core::generated::astria::sequencerblock::v1::sequencer_service_server::SequencerServiceServer;
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        eyre,
        OptionExt as _,
        Report,
        Result,
        WrapErr as _,
    },
};
use penumbra_tower_trace::{
    trace::request_span,
    v038::RequestExt as _,
};
use serde::{
    Deserialize,
    Serialize,
};
use telemetry::metrics::register_histogram_global;
use tendermint::v0_38::abci::ConsensusRequest;
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
};
use tower_abci::v038::Server;
use tracing::{
    error,
    error_span,
    info,
    info_span,
};
use url::Url;

use crate::{
    app::App,
    config::Config,
    grpc::sequencer::SequencerServer,
    ibc::host_interface::AstriaHost,
    mempool::Mempool,
    metrics::Metrics,
    service,
};

pub struct Sequencer;

impl Sequencer {
    #[expect(clippy::missing_errors_doc, reason = "not a public function")]
    pub async fn run_until_stopped(config: Config, metrics: &'static Metrics) -> Result<()> {
        cnidarium::register_metrics();
        register_histogram_global("cnidarium_get_raw_duration_seconds");
        register_histogram_global("cnidarium_nonverifiable_get_raw_duration_seconds");
        let span = info_span!("Sequencer::run_until_stopped");

        if config
            .db_filepath
            .try_exists()
            .context("failed checking for existence of db storage file")?
        {
            span.in_scope(|| {
                info!(
                    path = %config.db_filepath.display(),
                    "opening storage db"
                );
            });
        } else {
            span.in_scope(|| {
                info!(
                    path = %config.db_filepath.display(),
                    "creating storage db"
                );
            });
        }

        let mut signals = spawn_signal_handler();

        let substore_prefixes = vec![penumbra_ibc::IBC_SUBSTORE_PREFIX];

        let storage = cnidarium::Storage::load(
            config.db_filepath.clone(),
            substore_prefixes
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect(),
        )
        .await
        .map_err(anyhow_to_eyre)
        .wrap_err("failed to load storage backing chain state")?;
        let snapshot = storage.latest_snapshot();

        let mempool = Mempool::new(metrics, config.mempool_parked_max_tx_count);
        let app = App::new(snapshot, mempool.clone(), metrics)
            .await
            .wrap_err("failed to initialize app")?;

        let mempool_service = service::Mempool::new(storage.clone(), mempool.clone(), metrics);

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let (server_exit_tx, server_exit_rx) = tokio::sync::oneshot::channel();

        let grpc_addr = config
            .grpc_addr
            .parse()
            .wrap_err("failed to parse grpc_addr address")?;
        let grpc_server_handle = start_grpc_server(&storage, mempool, grpc_addr, shutdown_rx);

        span.in_scope(|| info!(%config.abci_listener_url, "starting abci sequencer"));
        let abci_server_handle = start_abci_server(
            &storage,
            app,
            mempool_service,
            config.abci_listener_url,
            server_exit_tx,
        )
        .wrap_err("failed to start ABCI server")?;

        select! {
            _ = signals.stop_rx.changed() => {
                span.in_scope(|| info!("shutting down sequencer"));
            }

            _ = server_exit_rx => {
                span.in_scope(|| error!("ABCI server task exited, this shouldn't happen"));
            }
        }

        shutdown_tx
            .send(())
            .map_err(|()| eyre!("failed to send shutdown signal to grpc server"))?;
        grpc_server_handle
            .await
            .wrap_err("grpc server task failed")?
            .wrap_err("grpc server failed")?;
        abci_server_handle.abort();
        Ok(())
    }
}

fn start_grpc_server(
    storage: &cnidarium::Storage,
    mempool: Mempool,
    grpc_addr: std::net::SocketAddr,
    shutdown_rx: oneshot::Receiver<()>,
) -> JoinHandle<Result<(), tonic::transport::Error>> {
    use futures::TryFutureExt as _;
    use ibc_proto::ibc::core::{
        channel::v1::query_server::QueryServer as ChannelQueryServer,
        client::v1::query_server::QueryServer as ClientQueryServer,
        connection::v1::query_server::QueryServer as ConnectionQueryServer,
    };
    use penumbra_tower_trace::remote_addr;
    use tower_http::cors::CorsLayer;

    let ibc = penumbra_ibc::component::rpc::IbcQuery::<AstriaHost>::new(storage.clone());
    let sequencer_api = SequencerServer::new(storage.clone(), mempool);
    let cors_layer: CorsLayer = CorsLayer::permissive();

    // TODO: setup HTTPS?
    let grpc_server = tonic::transport::Server::builder()
        .trace_fn(|req| {
            if let Some(remote_addr) = remote_addr(req) {
                let addr = remote_addr.to_string();
                error_span!("grpc", addr)
            } else {
                error_span!("grpc")
            }
        })
        // (from Penumbra) Allow HTTP/1, which will be used by grpc-web connections.
        // This is particularly important when running locally, as gRPC
        // typically uses HTTP/2, which requires HTTPS. Accepting HTTP/2
        // allows local applications such as web browsers to talk to pd.
        .accept_http1(true)
        // (from Penumbra) Add permissive CORS headers, so pd's gRPC services are accessible
        // from arbitrary web contexts, including from localhost.
        .layer(cors_layer)
        .add_service(ClientQueryServer::new(ibc.clone()))
        .add_service(ChannelQueryServer::new(ibc.clone()))
        .add_service(ConnectionQueryServer::new(ibc.clone()))
        .add_service(SequencerServiceServer::new(sequencer_api));

    info!(grpc_addr = grpc_addr.to_string(), "starting grpc server");
    tokio::task::spawn(
        grpc_server.serve_with_shutdown(grpc_addr, shutdown_rx.unwrap_or_else(|_| ())),
    )
}

fn start_abci_server(
    storage: &cnidarium::Storage,
    app: App,
    mempool_service: service::Mempool,
    listen_url: AbciListenUrl,
    server_exit_tx: oneshot::Sender<()>,
) -> Result<JoinHandle<()>, Report> {
    // Setup services required for the ABCI server
    let consensus_service = tower::ServiceBuilder::new()
        .layer(request_span::layer(|req: &ConsensusRequest| {
            req.create_span()
        }))
        .service(tower_actor::Actor::new(10, |queue: _| {
            let storage = storage.clone();
            async move { service::Consensus::new(storage, app, queue).run().await }
        }));
    let info_service =
        service::Info::new(storage.clone()).wrap_err("failed initializing info service")?;
    let snapshot_service = service::Snapshot;

    // Builds the server but does not start listening.
    let server = Server::builder()
        .consensus(consensus_service)
        .info(info_service)
        .mempool(mempool_service)
        .snapshot(snapshot_service)
        .finish()
        .ok_or_eyre("server builder didn't return server; are all fields set?")?;

    let server_handle = tokio::spawn(async move {
        let server_listen_result = match listen_url {
            AbciListenUrl::Tcp(socket_addr) => server.listen_tcp(socket_addr).await,
            AbciListenUrl::Uds(path) => server.listen_unix(path).await,
        };
        match server_listen_result {
            Ok(()) => {
                // this shouldn't happen, as there isn't a way for the ABCI server to exit
                info_span!("abci_server").in_scope(|| info!("ABCI server exited successfully"));
            }
            Err(e) => {
                error_span!("abci_server")
                    .in_scope(|| error!(err = e.as_ref(), "ABCI server exited with error"));
            }
        }
        let _ = server_exit_tx.send(());
    });

    Ok(server_handle)
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
                }
                _ = sigterm.recv() => {
                    info!("received SIGTERM");
                    let _ = stop_tx.send(());
                }
            }
        }
    });

    SignalReceiver {
        stop_rx,
    }
}

#[derive(Debug)]
pub enum AbciListenUrl {
    Tcp(SocketAddr),
    Uds(PathBuf),
}

impl Display for AbciListenUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AbciListenUrl::Tcp(socket_addr) => write!(f, "tcp://{socket_addr}"),
            AbciListenUrl::Uds(path) => write!(f, "uds://{}", path.display()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AbciListenUrlParseError {
    #[error(
        "parsed input as a tcp address `{parsed}`, but could not turn it into a socket address"
    )]
    TcpButBadSocketAddr { parsed: Url, source: std::io::Error },
    #[error("parsed input as a uds address `{parsed}`, but could not turn it into a path")]
    UdsButBadPath { parsed: Url },
    #[error(
        "parsed input as `{parsed}`, but scheme `scheme` is not suppported; supported schemes are \
         tcp, uds"
    )]
    UnsupportedScheme { parsed: Url, scheme: String },
    #[error("failed parsing input as URL")]
    Url {
        #[from]
        source: url::ParseError,
    },
}

impl FromStr for AbciListenUrl {
    type Err = AbciListenUrlParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let abci_url = Url::parse(s)?;

        match abci_url.scheme() {
            "uds" => {
                if let Ok(path) = abci_url.to_file_path() {
                    Ok(Self::Uds(path))
                } else {
                    Err(Self::Err::UdsButBadPath {
                        parsed: abci_url,
                    })
                }
            }
            "tcp" => match abci_url.socket_addrs(|| None) {
                Ok(mut socket_addrs) => {
                    let socket_addr = socket_addrs.pop().expect(
                        "the url crate is guaranteed to return vec with exactly one element \
                         because it relies on std::net::ToSocketAddrs::to_socket_addr; if this is \
                         no longer the case there was a breaking change in the url crate",
                    );
                    Ok(Self::Tcp(socket_addr))
                }
                Err(source) => {
                    return Err(Self::Err::TcpButBadSocketAddr {
                        parsed: abci_url,
                        source,
                    });
                }
            },
            // If more options are added here will also need to update the server startup
            // immediately below to support more than two protocols.
            other => Err(Self::Err::UnsupportedScheme {
                parsed: abci_url.clone(),
                scheme: other.to_string(),
            }),
        }
    }
}

impl<'de> Deserialize<'de> for AbciListenUrl {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = std::borrow::Cow::<'_, str>::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Serialize for AbciListenUrl {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}
