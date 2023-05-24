use std::error::Error;

use color_eyre::eyre::{
    eyre,
    Result,
};
use tendermint::abci::{
    ConsensusRequest,
    ConsensusResponse,
};
use tower_abci::Server;
use tower_actor::Actor;
use tracing::info;

use crate::{
    app::App,
    consensus::ConsensusService,
    info::InfoService,
    mempool::MempoolService,
    snapshot::SnapshotService,
};

pub struct Sequencer {
    #[allow(clippy::type_complexity)]
    server: Server<
        Actor<ConsensusRequest, ConsensusResponse, Box<dyn Error + Send + Sync>>,
        MempoolService,
        InfoService,
        SnapshotService,
    >,
}

impl Sequencer {
    pub async fn new() -> Result<Sequencer> {
        let storage = penumbra_storage::TempStorage::new()
            .await
            .map_err(|e| eyre!("should create temp storage; {}", e))?;
        let snapshot = storage.latest_snapshot();
        let app = App::new(snapshot);

        let consensus_service =
            tower::ServiceBuilder::new().service(tower_actor::Actor::new(10, |queue: _| {
                let storage = storage.clone();
                async move { ConsensusService::new(storage, app, queue).run().await }
            }));

        let info_service = InfoService::new(storage.clone());
        let mempool_service = MempoolService::new();
        let snapshot_service = SnapshotService::new();
        let server = Server::builder()
            .consensus(consensus_service)
            .info(info_service)
            .mempool(mempool_service)
            .snapshot(snapshot_service)
            .finish()
            .ok_or_else(|| eyre!("should build server"))?;

        Ok(Sequencer {
            server,
        })
    }

    pub async fn run(self, listen_addr: &str) {
        info!(?listen_addr, "starting sequencer");
        self.server
            .listen(listen_addr)
            .await
            .expect("should listen");
    }
}
