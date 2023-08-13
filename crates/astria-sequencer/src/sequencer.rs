use std::path::PathBuf;
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
    genesis::GenesisState,
    service,
};

pub struct Sequencer;

impl Sequencer {
    #[instrument(skip_all)]
    pub async fn run_until_stopped(config: Config) -> Result<()> {
        let genesis_state =
            GenesisState::from_path(config.genesis_file).context("failed reading genesis state")?;
        let db_filepath = PathBuf::from(config.db_datadir);
        let storage = penumbra_storage::Storage::load(db_filepath.clone())
            .await
            .context("failed to load storage backing chain state")?;
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);
        app.init_chain(genesis_state)
            .await
            .context("failed initializing app with genesis state")?;

        let consensus_service = tower::ServiceBuilder::new()
            .layer(request_span::layer(|req: &ConsensusRequest| {
                req.create_span()
            }))
            .service(tower_actor::Actor::new(10, |queue: _| {
                let storage = storage.clone();
                async move { service::Consensus::new(storage, app, queue).run().await }
            }));
        let mempool_service = service::Mempool;
        let info_service = service::Info::new(storage.clone());
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
