use std::sync::Arc;

use astria_sequencer::{
    app::App,
    consensus::ConsensusService,
    info::InfoService,
    mempool::MempoolService,
    snapshot::SnapshotService,
};
use tower_abci::Server;
use tracing::info;
use tracing_subscriber::EnvFilter;

pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:26658";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .init();

    let state = penumbra_storage::TempStorage::new()
        .await
        .expect("should create temp storage");
    let snapshot = state.snapshot(0).expect("should create snapshot");
    let app = Arc::new(App::new(snapshot).await.expect("should create app"));

    let consensus_service = ConsensusService::new(app);
    let info_service = InfoService::new();
    let mempool_service = MempoolService::new();
    let snapshot_service = SnapshotService::new();
    let server = Server::builder()
        .consensus(consensus_service)
        .info(info_service)
        .mempool(mempool_service)
        .snapshot(snapshot_service)
        .finish()
        .expect("should build server");

    info!("starting application listening on {}", DEFAULT_LISTEN_ADDR);
    server
        .listen(DEFAULT_LISTEN_ADDR)
        .await
        .expect("should listen");
}
