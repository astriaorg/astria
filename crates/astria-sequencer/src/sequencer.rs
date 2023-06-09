use std::error::Error;

use anyhow::{
    anyhow,
    Context as _,
};
use tendermint::abci::{
    ConsensusRequest,
    ConsensusResponse,
};
// TODO: update this to v037 for ABCI++ support
use tower_abci::v034::Server;
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
    pub async fn new() -> anyhow::Result<Sequencer> {
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
