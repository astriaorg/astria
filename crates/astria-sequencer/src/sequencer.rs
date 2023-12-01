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
use tower_abci::v037::Server;
use tracing::{
    info,
    instrument,
};

use crate::{
    app::App,
    config::Config,
    service,
    state_ext::StateReadExt as _,
};

// TODO: use penumbra's `IBC_SUBSTORE_PREFIX` after they merge #3419
const SUBSTORE_PREFIXES: [&str; 1] = ["ibc-data"];

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

        let storage = penumbra_storage::Storage::load(
            config.db_filepath.clone(),
            SUBSTORE_PREFIXES
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

        // TODO: config option for grpc bind address
        start_grpc_server(&storage, None)?;

        info!(config.listen_addr, "starting sequencer");
        server
            .listen_tcp(&config.listen_addr)
            .await
            .expect("should listen");
        Ok(())
    }
}

fn start_grpc_server(
    storage: &penumbra_storage::Storage,
    grpc_bind: Option<std::net::SocketAddr>,
) -> Result<()> {
    use ibc_proto::ibc::core::{
        channel::v1::query_server::QueryServer as ChannelQueryServer,
        client::v1::query_server::QueryServer as ClientQueryServer,
        connection::v1::query_server::QueryServer as ConnectionQueryServer,
    };
    use penumbra_tower_trace::remote_addr;
    use tonic::transport::Server;
    use tower_http::cors::CorsLayer;

    // gRPC server
    let ibc = penumbra_ibc::component::rpc::IbcQuery::new(storage.clone());
    // Set rather permissive CORS headers for pd's gRPC: the service
    // should be accessible from arbitrary web contexts, such as localhost,
    // or any FQDN that wants to reference its data.
    let cors_layer = CorsLayer::permissive();

    let grpc_server = Server::builder()
        .trace_fn(|req| {
            if let Some(remote_addr) = remote_addr(req) {
                tracing::error_span!("grpc", ?remote_addr)
            } else {
                tracing::error_span!("grpc")
            }
        })
        // Allow HTTP/1, which will be used by grpc-web connections.
        // This is particularly important when running locally, as gRPC
        // typically uses HTTP/2, which requires HTTPS. Accepting HTTP/2
        // allows local applications such as web browsers to talk to pd.
        .accept_http1(true)
        // Add permissive CORS headers, so pd's gRPC services are accessible
        // from arbitrary web contexts, including from localhost.
        .layer(cors_layer)
        .add_service(ClientQueryServer::new(ibc.clone()))
        .add_service(ChannelQueryServer::new(ibc.clone()))
        .add_service(ConnectionQueryServer::new(ibc.clone()));
    let grpc_bind = grpc_bind.unwrap_or(
        "127.0.0.1:8080"
            .parse()
            .context("failed to parse grpc_bind address")?,
    );
    tokio::task::Builder::new()
        .name("grpc_server")
        .spawn(grpc_server.serve(grpc_bind))
        .expect("failed to spawn grpc server");
    Ok(())
}
