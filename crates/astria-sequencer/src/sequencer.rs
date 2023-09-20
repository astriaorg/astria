use anyhow::{
    anyhow,
    Context as _,
    Result,
};
use penumbra_tower_trace::{
    trace::request_span,
    RequestExt as _,
};
use tendermint::abci::ConsensusRequest;
use tower_abci::v037::Server;
use tracing::{
    info,
    instrument,
};

use crate::{
    app::App,
    config::Config,
    service,
};

pub struct Sequencer;

impl Sequencer {
    #[instrument(skip_all)]
    pub async fn run_until_stopped(config: Config) -> Result<()> {
        if config
            .data_dir
            .try_exists()
            .context("failed checking for existence of db storage file")?
        {
            info!(
                path = %config.data_dir.display(),
                "opening storage db"
            );
        } else {
            info!(
                path = %config.data_dir.display(),
                "creating storage db"
            );
        }
        let storage = penumbra_storage::Storage::load(config.data_dir.clone())
            .await
            .context("failed to load storage backing chain state")?;
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
        let mempool_service = service::Mempool;
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

        info!(config.listen_addr, "starting sequencer");
        server
            .listen(&config.listen_addr)
            .await
            .expect("should listen");
        Ok(())
    }
}
