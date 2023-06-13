use anyhow::{
    anyhow,
    Context as _,
    Result,
};
use tower_abci::v037::Server;
use tracing::info;

use crate::{
    app::App,
    service,
};

pub struct Sequencer;

impl Sequencer {
    pub async fn run_until_stopped(listen_addr: &str) -> Result<()> {
        let storage = penumbra_storage::TempStorage::new()
            .await
            .context("failed to create temp storage backing chain state")?;
        let snapshot = storage.latest_snapshot();
        let app = App::new(snapshot);

        let consensus_service = tower::ServiceBuilder::new()
            .layer(request_span::layer(|req: &ConsensusRequest| {
                req.create_span()
            }))
            .service(tower_actor::Actor::new(10, |queue: _| {
                let storage = storage.clone();
                async move { service::Consensus::new(storage, app, queue).run().await }
            }));

        let info_service = service::Info::new(storage.clone());
        let mempool_service = service::Mempool;
        let snapshot_service = service::Snapshot::new();
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
