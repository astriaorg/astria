use anyhow::{
    anyhow,
    Context as _,
    Result,
};
use penumbra_tower_trace::{
    trace::request_span,
    v037::RequestExt as _,
};
use tendermint::v0_37::abci::ConsensusRequest;
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
};
use tower_abci::v037::Server;
use tracing::{
    error,
    info,
    instrument,
};

use crate::{
    app::App,
    config::Config,
    service,
    state_ext::StateReadExt as _,
};

pub struct Sequencer;

impl Sequencer {
    #[instrument(skip_all)]
    pub async fn run_until_stopped(config: Config) -> Result<()> {
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
        .context("failed to load storage backing chain state")?;
        let snapshot = storage.latest_snapshot();

        // the native asset should be configurable only at genesis.
        // the genesis state must include the native asset's base
        // denomination, and it is set in storage during init_chain.
        // on subsequent startups, we load the native asset from storage.
        if storage.latest_version() != u64::MAX {
            // native asset should be stored, fetch it
            let native_asset = snapshot
                .get_native_asset_denom()
                .await
                .context("failed to get native asset from storage")?;
            crate::asset::initialize_native_asset(&native_asset);
        }

        let app = App::new(snapshot);
        let consensus_service = tower::ServiceBuilder::new()
            .layer(request_span::layer(|req: &ConsensusRequest| {
                req.create_span()
            }))
            .service(tower_actor::Actor::new(10, |queue: _| {
                let storage = storage.clone();
                async move { service::Consensus::new(storage, app, queue).run().await }
            }));
        let mempool_service = service::Mempool::new(storage.clone());
        let info_service =
            service::Info::new(storage.clone()).context("failed initializing info service")?;
        let snapshot_service = service::Snapshot;

        let server = Server::builder()
            .consensus(consensus_service)
            .info(info_service)
            .mempool(mempool_service)
            .snapshot(snapshot_service)
            .finish()
            .ok_or_else(|| anyhow!("server builder didn't return server; are all fields set?"))?;

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let grpc_addr = config
            .grpc_addr
            .parse()
            .context("failed to parse grpc_addr address")?;
        start_grpc_server(&storage, grpc_addr, shutdown_rx)?;

        info!(config.listen_addr, "starting sequencer");
        select! {
            _ = signals.stop_rx.changed() => {
                info!("shutting down sequencer");
            }

            res = server.listen_tcp(&config.listen_addr) => {
                match res {
                    Ok(()) => {
                        // this shouldn't happen, as there isn't a way for the ABCI server to exit
                        info!("server exited successfully");
                    }
                    Err(e) => {
                        error!(?e, "server exited with error");
                    }
                }
            }
        }

        shutdown_tx
            .send(())
            .map_err(|()| anyhow!("failed to send shutdown signal to grpc server"))?;
        Ok(())
    }
}

fn start_grpc_server(
    storage: &cnidarium::Storage,
    grpc_addr: std::net::SocketAddr,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<()> {
    use futures::TryFutureExt as _;
    use ibc_proto::ibc::core::{
        channel::v1::query_server::QueryServer as ChannelQueryServer,
        client::v1::query_server::QueryServer as ClientQueryServer,
        connection::v1::query_server::QueryServer as ConnectionQueryServer,
    };
    use penumbra_tower_trace::remote_addr;
    use tonic::transport::Server;
    use tower_http::cors::CorsLayer;

    let ibc = penumbra_ibc::component::rpc::IbcQuery::new(storage.clone());
    let cors_layer = CorsLayer::permissive();

    // TODO: setup HTTPS?
    let grpc_server = Server::builder()
        .trace_fn(|req| {
            if let Some(remote_addr) = remote_addr(req) {
                tracing::error_span!("grpc", ?remote_addr)
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
        .add_service(ConnectionQueryServer::new(ibc.clone()));

    tokio::task::Builder::new()
        .name("grpc_server")
        .spawn(
            grpc_server.serve_with_shutdown(grpc_addr, shutdown_rx.map_ok_or_else(|_| (), |()| ())),
        )
        .context("failed to spawn grpc server")?;
    Ok(())
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
