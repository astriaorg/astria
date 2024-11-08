use std::time::Duration;

use astria_core::generated::{
    connect::{
        marketmap::v2::query_server::QueryServer as MarketMapQueryServer,
        oracle::v2::query_server::QueryServer as OracleQueryServer,
        service::v2::{
            oracle_client::OracleClient,
            QueryPricesRequest,
        },
    },
    sequencerblock::v1::sequencer_service_server::SequencerServiceServer,
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
    info,
    instrument,
    warn,
};

use crate::{
    address::StateReadExt as _,
    app::App,
    assets::StateReadExt as _,
    config::Config,
    grpc::sequencer::SequencerServer,
    ibc::host_interface::AstriaHost,
    mempool::Mempool,
    metrics::Metrics,
    service,
};

const MAX_RETRIES_TO_CONNECT_TO_ORACLE_SIDECAR: u32 = 36;

pub struct Sequencer;

impl Sequencer {
    #[instrument(skip_all)]
    pub async fn run_until_stopped(config: Config, metrics: &'static Metrics) -> Result<()> {
        cnidarium::register_metrics();
        register_histogram_global("cnidarium_get_raw_duration_seconds");
        register_histogram_global("cnidarium_nonverifiable_get_raw_duration_seconds");

        if config
            .db_filepath
            .try_exists()
            .context("failed checking for existence of db storage file")?
        {
            info!(
                path = %config.db_filepath.display(),
                "opening storage db"
            );
        } else {
            info!(
                path = %config.db_filepath.display(),
                "creating storage db"
            );
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

        let oracle_client = new_oracle_client(&config)
            .await
            .wrap_err("failed to create connected oracle client")?;

        let mempool = Mempool::new(metrics, config.mempool_parked_max_tx_count);
        let app = App::new(
            snapshot,
            mempool.clone(),
            crate::app::vote_extension::Handler::new(oracle_client),
            metrics,
        )
        .await
        .wrap_err("failed to initialize app")?;

        let consensus_service = tower::ServiceBuilder::new()
            .layer(request_span::layer(|req: &ConsensusRequest| {
                req.create_span()
            }))
            .service(tower_actor::Actor::new(10, |queue: _| {
                let storage = storage.clone();
                async move { service::Consensus::new(storage, app, queue).run().await }
            }));
        let mempool_service = service::Mempool::new(storage.clone(), mempool.clone(), metrics);
        let info_service =
            service::Info::new(storage.clone()).wrap_err("failed initializing info service")?;
        let snapshot_service = service::Snapshot;

        let server = Server::builder()
            .consensus(consensus_service)
            .info(info_service)
            .mempool(mempool_service)
            .snapshot(snapshot_service)
            .finish()
            .ok_or_eyre("server builder didn't return server; are all fields set?")?;

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let (server_exit_tx, server_exit_rx) = tokio::sync::oneshot::channel();

        let grpc_addr = config
            .grpc_addr
            .parse()
            .wrap_err("failed to parse grpc_addr address")?;
        let grpc_server_handle = start_grpc_server(&storage, mempool, grpc_addr, shutdown_rx);

        info!(config.listen_addr, "starting sequencer");
        let server_handle = tokio::spawn(async move {
            match server.listen_tcp(&config.listen_addr).await {
                Ok(()) => {
                    // this shouldn't happen, as there isn't a way for the ABCI server to exit
                    info!("ABCI server exited successfully");
                }
                Err(e) => {
                    error!(err = e.as_ref(), "ABCI server exited with error");
                }
            }
            let _ = server_exit_tx.send(());
        });

        select! {
            _ = signals.stop_rx.changed() => {
                info!("shutting down sequencer");
            }

            _ = server_exit_rx => {
                error!("ABCI server task exited, this shouldn't happen");
            }
        }

        shutdown_tx
            .send(())
            .map_err(|()| eyre!("failed to send shutdown signal to grpc server"))?;
        grpc_server_handle
            .await
            .wrap_err("grpc server task failed")?
            .wrap_err("grpc server failed")?;
        server_handle.abort();
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
    let market_map_api = crate::grpc::connect::SequencerServer::new(storage.clone());
    let oracle_api = crate::grpc::connect::SequencerServer::new(storage.clone());
    let cors_layer: CorsLayer = CorsLayer::permissive();

    // TODO: setup HTTPS?
    let grpc_server = tonic::transport::Server::builder()
        .trace_fn(|req| {
            if let Some(remote_addr) = remote_addr(req) {
                let addr = remote_addr.to_string();
                tracing::error_span!("grpc", addr)
            } else {
                tracing::error_span!("grpc")
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

/// Returns a new Connect oracle client or `Ok(None)` if `config.no_oracle` is true.
///
/// If `config.no_oracle` is false, returns `Ok(Some(...))` as soon as a successful response is
/// received from the oracle sidecar, or returns `Err` after a fixed number of failed re-attempts
/// (roughly equivalent to 5 minutes total).
#[instrument(skip_all, err)]
async fn new_oracle_client(config: &Config) -> Result<Option<OracleClient<Channel>>> {
    if config.no_oracle {
        return Ok(None);
    }
    let uri: Uri = config
        .oracle_grpc_addr
        .parse()
        .context("failed parsing oracle grpc address as Uri")?;
    let endpoint = Endpoint::from(uri.clone()).timeout(Duration::from_millis(
        config.oracle_client_timeout_milliseconds,
    ));

    let retry_config = tryhard::RetryFutureConfig::new(MAX_RETRIES_TO_CONNECT_TO_ORACLE_SIDECAR)
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
                    "failed to query oracle sidecar; retrying after backoff",
                );
                async {}
            },
        );

    let client = tryhard::retry_fn(|| connect_to_oracle(&endpoint, &uri))
        .with_config(retry_config)
        .await
        .wrap_err_with(|| {
            format!(
                "failed to query oracle sidecar after {MAX_RETRIES_TO_CONNECT_TO_ORACLE_SIDECAR} \
                 retries; giving up"
            )
        })?;
    Ok(Some(client))
}

#[instrument(skip_all, err(level = tracing::Level::WARN))]
async fn connect_to_oracle(endpoint: &Endpoint, uri: &Uri) -> Result<OracleClient<Channel>> {
    let mut oracle_client = OracleClient::new(
        endpoint
            .connect()
            .await
            .wrap_err("failed to connect to oracle sidecar")?,
    );
    let _ = oracle_client
        .prices(QueryPricesRequest::default())
        .await
        .wrap_err("oracle sidecar responded with error to query for prices");
    debug!(uri = %uri, "oracle sidecar is reachable");
    Ok(oracle_client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn should_wait_while_unable_to_connect() {
        // We only care about `no_oracle` and `oracle_grpc_addr` values - the others can be
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
            no_oracle: false,
            oracle_grpc_addr: "http://127.0.0.1:8081".to_string(),
            oracle_client_timeout_milliseconds: 1,
            mempool_parked_max_tx_count: 1,
        };

        let start = tokio::time::Instant::now();
        let error = new_oracle_client(&config).await.unwrap_err();
        assert!(start.elapsed() > Duration::from_secs(300));
        assert_eq!(
            error.to_string(),
            format!(
                "failed to query oracle sidecar after {MAX_RETRIES_TO_CONNECT_TO_ORACLE_SIDECAR} \
                 retries; giving up"
            )
        );
    }
}
