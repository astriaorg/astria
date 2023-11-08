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

        crate::asset::initialize_native_asset();

        let storage = penumbra_storage::Storage::load(config.db_filepath.clone())
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

        info!(config.listen_addr, "starting sequencer");
        server
            .listen_tcp(&config.listen_addr)
            .await
            .expect("should listen");
        Ok(())
    }
}
