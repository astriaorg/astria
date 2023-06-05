use std::error::Error;

use anyhow::{
    anyhow,
    Context as _,
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

/// The default address to listen on; this corresponds to the default ABCI
/// application address expected by tendermint.
pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:26658";

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

#[cfg(test)]
mod test {
    use super::*;

    use bytes::{BytesMut};
    use prost::Message;
    use tendermint_proto::abci as abci_pb;
    use tokio::net::TcpStream;

    #[tokio::test]
    async fn test_sequencer() {
        crate::telemetry::init(std::io::stdout).expect("failed to initialize telemetry");

        let sequencer = Sequencer::new().await.expect("should create sequencer");
        tokio::task::spawn(async move {
            sequencer.run(DEFAULT_LISTEN_ADDR).await;
        });

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let info_request = abci_pb::RequestInfo::default();

        let request = abci_pb::Request {
            value: Some(abci_pb::request::Value::Info(info_request)),
        };

        let stream = TcpStream::connect(DEFAULT_LISTEN_ADDR).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        stream.writable().await.unwrap();
        let buf = request.encode_length_delimited_to_vec();
        println!("encoded request: {:?}", buf);
        stream.try_write(&buf).unwrap();
        println!("wrote request");

        let mut buf = BytesMut::with_capacity(1024);
        stream.readable().await.unwrap();
        let _read_len = stream.try_read(&mut buf).unwrap();
        println!("read response: {:?}", buf);
        let resp = abci_pb::Response::decode_length_delimited(&mut buf).unwrap();
        println!("{:?}", resp);
    }
}
