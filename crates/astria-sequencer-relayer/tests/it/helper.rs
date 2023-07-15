use std::net::SocketAddr;

use astria_sequencer_client::BlockResponse;
use astria_sequencer_relayer::{
    api,
    config::Config,
    telemetry,
    types::SequencerBlockData,
    validator::Validator,
    SequencerRelayer,
};
use multiaddr::Multiaddr;
use once_cell::sync::Lazy;
use tempfile::NamedTempFile;
use tokio::{
    sync::{
        mpsc,
        oneshot,
    },
    task::JoinHandle,
};
use tracing::{
    debug,
    info,
};
use wiremock::MockServer;

static TELEMETRY: Lazy<()> = Lazy::new(|| {
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        telemetry::init(std::io::stdout, &filter_directives).unwrap();
    } else {
        telemetry::init(std::io::sink, "").unwrap();
    }
});

/// Copied verbatim from
/// [tendermint-rs](https://github.com/informalsystems/tendermint-rs/blob/main/config/tests/support/config/priv_validator_key.json)
const PRIVATE_VALIDATOR_KEY: &str = r#"
{
  "address": "AD7DAE5FEC609CF02F9BDE7D81D0C3CD66141563",
  "pub_key": {
    "type": "tendermint/PubKeyEd25519",
    "value": "8mv0sqLoTOt6U8PxrndAh3myAGR4L7rb3w42WVnuRTQ="
  },
  "priv_key": {
    "type": "tendermint/PrivKeyEd25519",
    "value": "skHDGUYe2pOhwfSrXZQ6KeKnmKgTOn+f++Vmj4OOqIHya/SyouhM63pTw/Gud0CHebIAZHgvutvfDjZZWe5FNA=="
  }
}
"#;

pub struct TestSequencerRelayer {
    /// The socket address that sequencer relayer is serving its API endpoint on
    ///
    /// This is useful for checking if it's healthy, ready, or how many p2p peers
    /// are subscribed to it.
    pub api_address: SocketAddr,

    /// The mocked celestia node jsonrpc server
    pub celestia: MockCelestia,

    /// The mocked astria conductor gossip node receiving gossiped messages
    pub conductor: MockConductor,

    pub original_block_response: BlockResponse,

    /// The mocked sequencer service (also serving as tendermint jsonrpc?)
    pub sequencer: MockServer,

    pub sequencer_relayer: JoinHandle<()>,

    pub _keyfile: NamedTempFile,

    pub config: Config,
}

pub async fn spawn_sequencer_relayer() -> TestSequencerRelayer {
    Lazy::force(&TELEMETRY);
    let mut conductor = MockConductor::start();
    let conductor_bootnode = (&mut conductor.bootnode_rx).await.unwrap();

    let mut celestia = MockCelestia::start().await;
    let celestia_addr = (&mut celestia.addr_rx).await.unwrap();

    let (keyfile, validator) = tokio::task::spawn_blocking(|| {
        use std::io::Write as _;

        let keyfile = NamedTempFile::new().unwrap();
        (&keyfile)
            .write_all(PRIVATE_VALIDATOR_KEY.as_bytes())
            .unwrap();
        let validator = Validator::from_path(&keyfile).unwrap();
        (keyfile, validator)
    })
    .await
    .unwrap();

    let block_response = create_block_response(validator);
    let sequencer = start_mocked_sequencer(block_response.clone()).await;

    let config = {
        let mut cfg = Config::default();
        cfg.bootnodes = Some(vec![conductor_bootnode.to_string()]);
        cfg.celestia_endpoint = format!("http://{celestia_addr}");
        cfg.sequencer_endpoint = sequencer.uri();
        cfg.rpc_port = 0;
        cfg.p2p_port = 0;
        cfg.validator_key_file = keyfile.path().to_string_lossy().to_string();
        cfg
    };
    info!(config = serde_json::to_string(&config).unwrap());
    let config_clone = config.clone();
    let sequencer_relayer = tokio::task::spawn_blocking(|| SequencerRelayer::new(config_clone))
        .await
        .unwrap()
        .unwrap();
    let api_address = sequencer_relayer.local_addr();
    let sequencer_relayer = tokio::task::spawn(sequencer_relayer.run());

    TestSequencerRelayer {
        api_address,
        celestia,
        conductor,
        config,
        original_block_response: block_response,
        sequencer,
        sequencer_relayer,
        _keyfile: keyfile,
    }
}

pub async fn get_api_status(addr: SocketAddr) -> api::Status {
    reqwest::get(format!("http://{addr}/status"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

pub async fn loop_until_conductor_has_subscribed(addr: SocketAddr) {
    loop {
        let state = get_api_status(addr).await;
        let num_peers = state.number_of_subscribed_peers.unwrap();
        if num_peers > 0 {
            break;
        }
    }
}

use astria_celestia_jsonrpc_client::rpc_impl::{
    blob::Blob,
    state::{
        Fee,
        StateServer,
    },
};
use jsonrpsee::{
    core::async_trait,
    server::ServerHandle,
    types::ErrorObjectOwned,
};

pub struct MockCelestia {
    pub addr_rx: oneshot::Receiver<SocketAddr>,
    pub rpc_confirmed_rx: mpsc::UnboundedReceiver<Vec<Blob>>,
    pub _server_handle: ServerHandle,
}

impl MockCelestia {
    async fn start() -> Self {
        use jsonrpsee::server::ServerBuilder;
        let (addr_tx, addr_rx) = oneshot::channel();
        let server = ServerBuilder::default().build("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        addr_tx.send(addr).unwrap();
        let (rpc_confirmed_tx, rpc_confirmed_rx) = mpsc::unbounded_channel();
        let celestia = CelestiaImpl {
            rpc_confirmed_tx,
        };
        let _server_handle = server.start(celestia.into_rpc()).unwrap();
        Self {
            addr_rx,
            rpc_confirmed_rx,
            _server_handle,
        }
    }
}

struct CelestiaImpl {
    rpc_confirmed_tx: mpsc::UnboundedSender<Vec<Blob>>,
}

#[async_trait]
impl StateServer for CelestiaImpl {
    async fn submit_pay_for_blob(
        &self,
        _fee: Fee,
        _gas_limit: u64,
        blobs: Vec<Blob>,
    ) -> Result<Box<serde_json::value::RawValue>, ErrorObjectOwned> {
        use astria_celestia_jsonrpc_client::state::SubmitPayForBlobResponse;
        use serde_json::value::RawValue;

        self.rpc_confirmed_tx.send(blobs).unwrap();

        let rsp = RawValue::from_string(
            serde_json::to_string(&SubmitPayForBlobResponse {
                height: 100,
                rest: serde_json::Value::Null,
            })
            .unwrap(),
        )
        .unwrap();
        Ok(rsp)
    }
}

pub struct MockConductor {
    pub bootnode_rx: oneshot::Receiver<Multiaddr>,
    pub block_rx: oneshot::Receiver<SequencerBlockData>,
    pub task: JoinHandle<()>,
}

impl MockConductor {
    fn start() -> Self {
        use astria_gossipnet::{
            network::{
                Keypair,
                NetworkBuilder,
                Sha256Topic,
            },
            network_stream::Event,
        };
        let mut conductor = NetworkBuilder::new()
            .keypair(Keypair::generate_ed25519())
            .with_mdns(false)
            .build()
            .unwrap();
        let (bootnode_tx, bootnode_rx) = oneshot::channel();
        let (block_tx, block_rx) = oneshot::channel();
        let task = tokio::task::spawn(async move {
            use futures::StreamExt as _;
            let event = conductor
                .next()
                .await
                .expect("should have received an event from gossip network")
                .expect("event should not have been an error");

            // The first event should be that we start listening
            match event {
                Event::NewListenAddr(_) => {
                    let multiaddr = conductor.multiaddrs()[0].clone();
                    debug!(?multiaddr, "conductor is listening");
                    bootnode_tx.send(multiaddr).unwrap();
                }
                other => panic!("event other than NewListerAddr received: {other:?}"),
            };

            // Wait until sequencer-relayer connects. We have to wait with subscribing to
            // "blocks" until after the connection is established because otherwise we
            // have to wait until the next gossipsub heartbeat (which is configured to be
            // 10 seconds).
            loop {
                let Some(event) = conductor.next().await else {
                    break;
                };
                match event.unwrap() {
                    Event::GossipsubPeerConnected(peer_id) => {
                        debug!(?peer_id, "conductor connected to peer");
                        break;
                    }
                    _ => {}
                }
            }

            let topic = Sha256Topic::new("blocks");
            debug!(topic.hash = %topic.hash(), "subscribing to topic");
            conductor.subscribe(&Sha256Topic::new("blocks"));

            loop {
                let Some(event) = conductor.next().await else {
                    break;
                };

                match event.unwrap() {
                    Event::GossipsubPeerConnected(peer_id) => {
                        debug!(?peer_id, "conductor connected to peer");
                    }
                    Event::GossipsubPeerSubscribed(peer_id, topic_hash) => {
                        debug!(?peer_id, ?topic_hash, "remote peer subscribed to topic");
                    }
                    Event::GossipsubMessage(msg) => {
                        debug!(?msg, "conductor received message");
                        let block = SequencerBlockData::from_bytes(&msg.data).unwrap();
                        block_tx.send(block).unwrap();
                        break;
                    }
                    _ => {}
                }
            }
        });
        Self {
            task,
            block_rx,
            bootnode_rx,
        }
    }
}

fn create_block_response(validator: Validator) -> BlockResponse {
    use astria_sequencer::{
        accounts::types::Nonce,
        transaction::{
            action::Action,
            Unsigned,
        },
    };
    use tendermint::{
        block,
        chain,
        evidence,
        hash::AppHash,
        merkle::simple_hash_from_byte_vectors,
        Block,
        Hash,
        Time,
    };
    let signing_key = validator.signing_key();

    let signed_tx = Unsigned::new_with_actions(
        Nonce::from(1),
        vec![Action::new_sequence_action(
            b"test_chain_id".to_vec(),
            b"hello_world".to_vec(),
        )],
    )
    .into_signed(&signing_key);
    let data = vec![signed_tx.to_bytes()];
    let data_hash = Some(Hash::Sha256(simple_hash_from_byte_vectors::<sha2::Sha256>(
        &data,
    )));

    BlockResponse {
        block_id: block::Id {
            hash: Hash::Sha256([0; 32]),
            part_set_header: block::parts::Header::new(0, Hash::None).unwrap(),
        },
        block: Block::new(
            block::Header {
                version: block::header::Version {
                    block: 0,
                    app: 0,
                },
                chain_id: chain::Id::try_from("test").unwrap(),
                height: block::Height::from(1u32),
                time: Time::now(),
                last_block_id: None,
                last_commit_hash: None,
                data_hash,
                validators_hash: Hash::Sha256([0; 32]),
                next_validators_hash: Hash::Sha256([0; 32]),
                consensus_hash: Hash::Sha256([0; 32]),
                app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
                last_results_hash: None,
                evidence_hash: None,
                proposer_address: validator.address().clone(),
            },
            data,
            evidence::List::default(),
            None,
        )
        .unwrap(),
    }
}

// async fn start_mocked_sequencer(validator: Validator) -> MockServer {
async fn start_mocked_sequencer(response: BlockResponse) -> MockServer {
    use serde_json::json;
    use tendermint_rpc::{
        response::Wrapper,
        Id,
    };
    use wiremock::{
        matchers::body_partial_json,
        Mock,
        ResponseTemplate,
    };
    let server = MockServer::start().await;
    let response = Wrapper::new_with_id(Id::Num(1), Some(response), None);
    Mock::given(body_partial_json(json!({"method": "block"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;
    server
}
