use std::time::Duration;

use astria_core::{
    generated::{
        astria::sequencerblock::v1::sequencer_service_server::SequencerServiceServer,
        price_feed::{
            marketmap::v2::query_server::QueryServer as MarketMapQueryServer,
            oracle::v2::query_server::QueryServer as OracleQueryServer,
            service::v2::oracle_client::OracleClient,
        },
    },
    upgrades::v1::Upgrades,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        self,
        eyre,
        OptionExt as _,
        Result,
        WrapErr as _,
    },
};
use penumbra_tower_trace::{
    trace::request_span,
    v038::RequestExt as _,
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
use tonic::transport::{
    Channel,
    Endpoint,
    Uri,
};
use tower_abci::v038::Server;
use tracing::{
    debug,
    error,
    error_span,
    info,
    info_span,
    instrument,
    warn,
};

use crate::{
    address::StateReadExt as _,
    app::{
        App,
        ShouldShutDown,
    },
    assets::StateReadExt as _,
    config::Config,
    grpc::sequencer::SequencerServer,
    ibc::host_interface::AstriaHost,
    mempool::Mempool,
    metrics::Metrics,
    service,
    upgrades::UpgradesHandler,
};

const MAX_RETRIES_TO_CONNECT_TO_PRICE_FEED_SIDECAR: u32 = 36;

pub struct Sequencer;

type GrpcServerHandle = JoinHandle<Result<(), tonic::transport::Error>>;
type AbciServerHandle = JoinHandle<()>;

struct RunningGrpcServer {
    pub handle: GrpcServerHandle,
    pub shutdown_tx: oneshot::Sender<()>,
}

struct RunningAbciServer {
    pub handle: AbciServerHandle,
    pub shutdown_rx: oneshot::Receiver<()>,
}

enum InitializationOutcome {
    Initialized {
        grpc_server: RunningGrpcServer,
        abci_server: RunningAbciServer,
    },
    ShutDownForUpgrade,
}

impl Sequencer {
    /// Builds and runs the sequencer until it is either stopped by a signal or an error occurs.
    ///
    /// # Errors
    /// Returns an error in the following cases:
    /// - Database file does not exist, or cannot be loaded into storage
    /// - The app fails to initialize
    /// - Info service fails to initialize
    /// - The server builder fails to return a server
    /// - The gRPC address cannot be parsed
    /// - The gRPC server fails to exit properly
    pub async fn spawn(config: Config, metrics: &'static Metrics) -> Result<()> {
        let mut signals = spawn_signal_handler();
        let initialize_fut = Self::initialize(config, metrics);
        select! {
            _ = signals.stop_rx.changed() => {
                info_span!("initialize").in_scope(|| info!("shutting down sequencer"));
                Ok(())
            }

            result = initialize_fut => {
                match result? {
                    InitializationOutcome::Initialized{grpc_server,abci_server  } => {
                        Self::run_until_stopped(grpc_server, abci_server, &mut signals).await
                    }
                    InitializationOutcome::ShutDownForUpgrade => { Ok(()) }
                }
            }
        }
    }

    async fn run_until_stopped(
        grpc_server: RunningGrpcServer,
        abci_server: RunningAbciServer,
        signals: &mut SignalReceiver,
    ) -> Result<()> {
        select! {
            _ = signals.stop_rx.changed() => {
                info_span!("run_until_stopped").in_scope(|| info!("shutting down sequencer"));
            }

            _ = abci_server.shutdown_rx => {
                info_span!("run_until_stopped").in_scope(|| error!("ABCI server task exited, this shouldn't happen"));
            }
        }

        grpc_server
            .shutdown_tx
            .send(())
            .map_err(|()| eyre!("failed to send shutdown signal to grpc server"))?;
        grpc_server
            .handle
            .await
            .wrap_err("grpc server task failed")?
            .wrap_err("grpc server failed")?;
        abci_server.handle.abort();
        Ok(())
    }

    #[instrument(skip_all)]
    async fn initialize(
        config: Config,
        metrics: &'static Metrics,
    ) -> Result<InitializationOutcome> {
        cnidarium::register_metrics();
        register_histogram_global("cnidarium_get_raw_duration_seconds");
        register_histogram_global("cnidarium_nonverifiable_get_raw_duration_seconds");

        let substore_prefixes = vec![penumbra_ibc::IBC_SUBSTORE_PREFIX];

        let storage = cnidarium::Storage::load(
            config.db_filepath.clone(),
            substore_prefixes
                .into_iter()
                .map(ToString::to_string)
                .collect(),
        )
        .await
        .map_err(anyhow_to_eyre)
        .wrap_err("failed to load storage backing chain state")?;
        let snapshot = storage.latest_snapshot();

        let upgrades_handler =
            UpgradesHandler::new(&config.upgrades_filepath, config.cometbft_rpc_addr.clone())
                .wrap_err("failed constructing upgrades handler")?;
        upgrades_handler
            .ensure_historical_upgrades_applied(&snapshot)
            .await
            .wrap_err("historical upgrades not applied")?;
        if let ShouldShutDown::ShutDownForUpgrade {
            upgrade_activation_height,
            block_time,
            hex_encoded_app_hash,
        } = upgrades_handler
            .should_shut_down(&snapshot)
            .await
            .wrap_err("failed to establish if sequencer should shut down for upgrade")?
        {
            info!(
                upgrade_activation_height,
                latest_app_hash = %hex_encoded_app_hash,
                latest_block_time = %block_time,
                "shutting down for upgrade"
            );
            return Ok(InitializationOutcome::ShutDownForUpgrade);
        }

        // the native asset should be configurable only at genesis.
        // the genesis state must include the native asset's base
        // denomination, and it is set in storage during init_chain.
        // on subsequent startups, we load the native asset from storage.
        if storage.latest_version() != u64::MAX {
            let _ = snapshot
                .get_native_asset()
                .await
                .context("failed to query state for native asset")?;
            let _ = snapshot
                .get_base_prefix()
                .await
                .context("failed to query state for base prefix")?;
        }

        let mempool = Mempool::new(metrics, config.mempool_parked_max_tx_count);
        let price_feed_client = new_price_feed_client(&config)
            .await
            .wrap_err("failed to create connected price feed client")?;
        let upgrades = upgrades_handler.upgrades().clone();
        let app = App::new(
            snapshot,
            mempool.clone(),
            upgrades_handler,
            crate::app::vote_extension::Handler::new(price_feed_client),
            metrics,
        )
        .await
        .wrap_err("failed to initialize app")?;

        let consensus_token = tokio_util::sync::CancellationToken::new();
        let cloned_token = consensus_token.clone();
        let consensus_service = tower::ServiceBuilder::new()
            .layer(request_span::layer(|req: &ConsensusRequest| {
                req.create_span()
            }))
            .service(tower_actor::Actor::new(10, |queue: _| {
                let storage = storage.clone();
                async move {
                    service::Consensus::new(storage, app, queue, cloned_token)
                        .run()
                        .await
                }
            }));
        let mempool_service = service::Mempool::new(storage.clone(), mempool.clone(), metrics);
        let info_service =
            service::Info::new(storage.clone()).wrap_err("failed initializing info service")?;
        let snapshot_service = service::Snapshot;

        let abci_server = Server::builder()
            .consensus(consensus_service)
            .info(info_service)
            .mempool(mempool_service)
            .snapshot(snapshot_service)
            .finish()
            .ok_or_eyre("server builder didn't return server; are all fields set?")?;

        let (grpc_shutdown_tx, grpc_shutdown_rx) = tokio::sync::oneshot::channel();
        let (abci_shutdown_tx, abci_shutdown_rx) = tokio::sync::oneshot::channel();

        let grpc_addr = config
            .grpc_addr
            .parse()
            .wrap_err("failed to parse grpc_addr address")?;
        let grpc_server_handle =
            start_grpc_server(&storage, mempool, upgrades, grpc_addr, grpc_shutdown_rx);

        debug!(config.listen_addr, "starting sequencer");

        let listen_addr = config.listen_addr.clone();
        let abci_server_handle = tokio::spawn(async move {
            match abci_server.listen_tcp(listen_addr).await {
                Ok(()) => {
                    // this shouldn't happen, as there isn't a way for the ABCI server to exit
                    info_span!("abci_server").in_scope(|| info!("ABCI server exited successfully"));
                }
                Err(e) => {
                    error_span!("abci_server")
                        .in_scope(|| error!(err = e.as_ref(), "ABCI server exited with error"));
                }
            }
            let _ = abci_shutdown_tx.send(());
        });

        let grpc_server = RunningGrpcServer {
            handle: grpc_server_handle,
            shutdown_tx: grpc_shutdown_tx,
        };
        let abci_server = RunningAbciServer {
            handle: abci_server_handle,
            shutdown_rx: abci_shutdown_rx,
        };

        Ok(InitializationOutcome::Initialized {
            grpc_server,
            abci_server,
        })
    }
}

fn start_grpc_server(
    storage: &cnidarium::Storage,
    mempool: Mempool,
    upgrades: Upgrades,
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
    let sequencer_api = SequencerServer::new(storage.clone(), mempool, upgrades);
    let market_map_api = crate::grpc::price_feed::SequencerServer::new(storage.clone());
    let oracle_api = crate::grpc::price_feed::SequencerServer::new(storage.clone());
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
        .add_service(SequencerServiceServer::new(sequencer_api))
        .add_service(MarketMapQueryServer::new(market_map_api))
        .add_service(OracleQueryServer::new(oracle_api));

    info!(grpc_addr = grpc_addr.to_string(), "starting grpc server");
    tokio::task::spawn(
        grpc_server.serve_with_shutdown(grpc_addr, shutdown_rx.unwrap_or_else(|_| ())),
    )
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

/// Returns a new price feed client or `Ok(None)` if `config.no_price_feed` is true.
///
/// If `config.no_price_feed` is false, returns `Ok(Some(...))` as soon as a successful response is
/// received from the price feed sidecar, or returns `Err` after a fixed number of failed
/// re-attempts (roughly equivalent to 5 minutes total).
#[instrument(skip_all, err)]
async fn new_price_feed_client(config: &Config) -> Result<Option<OracleClient<Channel>>> {
    if config.no_price_feed {
        return Ok(None);
    }
    let uri: Uri = config
        .price_feed_grpc_addr
        .parse()
        .context("failed parsing price feed grpc address as Uri")?;
    let endpoint = Endpoint::from(uri.clone()).timeout(Duration::from_millis(
        config.price_feed_client_timeout_milliseconds,
    ));

    let retry_config =
        tryhard::RetryFutureConfig::new(MAX_RETRIES_TO_CONNECT_TO_PRICE_FEED_SIDECAR)
            .exponential_backoff(Duration::from_millis(100))
            .max_delay(Duration::from_secs(10))
            .on_retry(
                |attempt, next_delay: Option<Duration>, error: &eyre::Report| {
                    let wait_duration = next_delay
                        .map(humantime::format_duration)
                        .map(tracing::field::display);
                    warn!(
                        error = error.as_ref() as &dyn std::error::Error,
                        attempt,
                        wait_duration,
                        "failed to query price feed oracle sidecar; retrying after backoff",
                    );
                    async {}
                },
            );

    let client = tryhard::retry_fn(|| connect_to_price_feed_sidecar(&endpoint, &uri))
        .with_config(retry_config)
        .await
        .wrap_err_with(|| {
            format!(
                "failed to query price feed sidecar after \
                 {MAX_RETRIES_TO_CONNECT_TO_PRICE_FEED_SIDECAR} retries; giving up"
            )
        })?;
    Ok(Some(client))
}

#[instrument(skip_all, err(level = tracing::Level::WARN))]
async fn connect_to_price_feed_sidecar(
    endpoint: &Endpoint,
    uri: &Uri,
) -> Result<OracleClient<Channel>> {
    let client = OracleClient::new(
        endpoint
            .connect()
            .await
            .wrap_err("failed to connect to price feed sidecar")?,
    );
    debug!(uri = %uri, "price feed sidecar is reachable");
    Ok(client)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[tokio::test(start_paused = true)]
    async fn should_wait_while_unable_to_connect() {
        // We only care about `no_price_feed` and `price_feed_grpc_addr` values - the others can be
        // meaningless.
        let config = Config {
            listen_addr: String::new(),
            db_filepath: "".into(),
            log: String::new(),
            grpc_addr: String::new(),
            force_stdout: true,
            no_otel: true,
            no_metrics: true,
            metrics_http_listener_addr: String::new(),
            pretty_print: true,
            upgrades_filepath: PathBuf::new(),
            cometbft_rpc_addr: String::new(),
            no_price_feed: false,
            price_feed_grpc_addr: "http://127.0.0.1:8081".to_string(),
            price_feed_client_timeout_milliseconds: 1,
            mempool_parked_max_tx_count: 1,
        };

        let start = tokio::time::Instant::now();
        let error = new_price_feed_client(&config).await.unwrap_err();
        assert!(start.elapsed() > Duration::from_secs(300));
        assert_eq!(
            error.to_string(),
            format!(
                "failed to query price feed sidecar after \
                 {MAX_RETRIES_TO_CONNECT_TO_PRICE_FEED_SIDECAR} retries; giving up"
            )
        );
    }
}
