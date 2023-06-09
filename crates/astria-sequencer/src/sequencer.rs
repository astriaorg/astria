use anyhow::{
    anyhow,
    Context as _,
    Result,
};
use tower_abci::v037::Server;
use tracing::info;

use crate::{
    app::App,
    consensus::ConsensusService,
    info::InfoService,
    mempool::MempoolService,
    snapshot::SnapshotService,
};

pub struct Sequencer;

impl Sequencer {
    pub async fn run_until_stopped(listen_addr: &str) -> Result<()> {
        let storage = penumbra_storage::TempStorage::new()
            .await
            .context("failed to create temp storage backing chain state")?;
        let snapshot = storage.latest_snapshot();
        let app = App::new(snapshot);

        let consensus_service =
            tower::ServiceBuilder::new().service(tower_actor::Actor::new(10, |queue: _| {
                let storage = storage.clone();
                async move { ConsensusService::new(storage, app, queue).run().await }
            }));

        let info_service = InfoService::new(storage.clone());
        let mempool_service = MempoolService;
        let snapshot_service = SnapshotService::new();
        let server = Server::builder()
            .consensus(consensus_service)
            .info(info_service)
            .mempool(mempool_service)
            .snapshot(snapshot_service)
            .finish()
            .ok_or_else(|| anyhow!("server builder didn't return server; are all fields set?"))?;

        info!(?listen_addr, "starting sequencer");
        server.listen(listen_addr).await.expect("should listen");
        Ok(())
    }
}
