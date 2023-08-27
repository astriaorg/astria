use std::{
    net::SocketAddr,
    time::Duration,
};

use astria_sequencer_relayer::{
    config::{
        Config,
        MAX_RELAYER_QUEUE_TIME_MS,
    },
    telemetry,
    validator::Validator,
    SequencerRelayer,
};
use astria_sequencer_types::SequencerBlockData;
use multiaddr::Multiaddr;
use once_cell::sync::Lazy;
use serde_json::json;
use tempfile::NamedTempFile;
use tendermint_rpc::{
    endpoint::block::Response,
    response::Wrapper,
    Id,
};
use tokio::{
    sync::{
        mpsc,
        oneshot,
    },
    task::JoinHandle,
    time::{
        self,
        Instant,
    },
};
use tracing::{
    debug,
    info,
};
use wiremock::{
    matchers::body_partial_json,
    Mock,
    MockServer,
    ResponseTemplate,
};

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

    /// The mocked sequencer service (also serving as tendermint jsonrpc?), comet bft
    pub sequencer: MockServer,

    pub sequencer_relayer: JoinHandle<()>,

    pub config: Config,

    pub validator: Validator,

    pub _keyfile: NamedTempFile,
}

impl TestSequencerRelayer {
    /// Polling another future than the tick interval in relayer `stop_msg` loop is controlled by
    /// advancing time mod block time is not 0.
    pub async fn advance_to_time_mod_block_time_not_zero(&self, ms: u64) {
        let millis = Duration::from_millis(ms);
        if (Instant::now().elapsed() + millis).as_millis() as u64 % self.config.block_time == 0 {
            panic!("time mod block time is zero, exactly on tick interval")
        }
        time::advance(millis).await;
    }

    /// Sequencer is polled for new blocks every sequencer block time.
    pub async fn advance_time_by_n_sequencer_ticks(&self, n: u64) {
        time::advance(Duration::from_millis(n * self.config.block_time)).await;
    }
}

/// Latency in celestia network.
pub enum CelestiaMode {
    /// Celestia network with no delay.
    Immediate,
    /// A delayed celestia network. Takes the time to delay in ms after finalization (when
    /// submission to da occurs) before celestia client sees blobs on celestia network.
    DelayedSinceFinalization(u64),
}

/// Spawn a [`TestSequencerRelayer`].
pub async fn spawn_sequencer_relayer(
    mut config: Config,
    celestia_mode: CelestiaMode,
) -> TestSequencerRelayer {
    Lazy::force(&TELEMETRY);

    let mut conductor = MockConductor::start();
    let conductor_bootnode = (&mut conductor.bootnode_rx).await.unwrap();

    // mock celestia use the sequencer block time to calculate artificial delay. real celestia has
    // longer block times than the sequencer block time.
    let mut celestia = MockCelestia::start(celestia_mode).await;
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

    let sequencer = start_mocked_sequencer().await;

    config.bootnodes = Some(vec![conductor_bootnode.to_string()]);
    config.celestia_endpoint = format!("http://{celestia_addr}");
    config.sequencer_endpoint = sequencer.uri();
    config.rpc_port = 0;
    config.p2p_port = 0;
    config.validator_key_file = keyfile.path().to_string_lossy().to_string();

    info!(config = serde_json::to_string(&config).unwrap());
    let config_clone = config.clone();
    let sequencer_relayer = tokio::task::spawn_blocking(|| SequencerRelayer::new(config_clone))
        .await
        .unwrap()
        .unwrap();
    let api_address = sequencer_relayer.local_addr();
    let sequencer_relayer = tokio::task::spawn(sequencer_relayer.run());

    loop_until_sequencer_relayer_is_ready(api_address).await;

    TestSequencerRelayer {
        api_address,
        celestia,
        conductor,
        config,
        sequencer,
        sequencer_relayer,
        validator,
        _keyfile: keyfile,
    }
}

pub async fn loop_until_sequencer_relayer_is_ready(addr: SocketAddr) {
    #[derive(Debug, serde::Deserialize)]
    struct Readyz {
        status: String,
    }
    loop {
        let readyz = reqwest::get(format!("http://{addr}/readyz"))
            .await
            .unwrap()
            .json::<Readyz>()
            .await
            .unwrap();
        if readyz.status.to_lowercase() == "ok" {
            break;
        }
    }
    #[derive(Debug, serde::Deserialize)]
    struct Status {
        number_of_subscribed_peers: u64,
    }
    loop {
        let status = reqwest::get(format!("http://{addr}/status"))
            .await
            .unwrap()
            .json::<Status>()
            .await
            .unwrap();
        if status.number_of_subscribed_peers > 0 {
            break;
        }
    }
}

use astria_celestia_jsonrpc_client::rpc_impl::{
    blob::Blob,
    header::HeaderServer,
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
    pub state_rpc_confirmed_rx: mpsc::UnboundedReceiver<Vec<Blob>>,
    pub _server_handle: ServerHandle,
}

impl MockCelestia {
    async fn start(mode: CelestiaMode) -> Self {
        use jsonrpsee::server::ServerBuilder;
        let (addr_tx, addr_rx) = oneshot::channel();
        let server = ServerBuilder::default().build("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        addr_tx.send(addr).unwrap();
        let (state_rpc_confirmed_tx, state_rpc_confirmed_rx) = mpsc::unbounded_channel();

        let state_celestia = StateCelestiaImpl {
            mode,
            rpc_confirmed_tx: state_rpc_confirmed_tx,
        };
        let header_celestia = HeaderCelestiaImpl {};
        let mut merged_celestia = state_celestia.into_rpc();
        merged_celestia.merge(header_celestia.into_rpc()).unwrap();
        let _server_handle = server.start(merged_celestia);
        Self {
            addr_rx,
            state_rpc_confirmed_rx,
            _server_handle,
        }
    }
}

struct HeaderCelestiaImpl;

#[async_trait]
impl HeaderServer for HeaderCelestiaImpl {
    async fn network_head(&self) -> Result<Box<serde_json::value::RawValue>, ErrorObjectOwned> {
        use astria_celestia_jsonrpc_client::header::{
            Commit,
            NetworkHeaderResponse,
        };
        use serde_json::{
            to_string,
            value::RawValue,
            Value,
        };
        let rsp = RawValue::from_string(
            to_string(&NetworkHeaderResponse {
                commit: Commit {
                    height: 42,
                    rest: Value::default(),
                },
                inner: Value::default(),
            })
            .unwrap(),
        )
        .unwrap();
        Ok(rsp)
    }
}

struct StateCelestiaImpl {
    mode: CelestiaMode,
    rpc_confirmed_tx: mpsc::UnboundedSender<Vec<Blob>>,
}

#[async_trait]
impl StateServer for StateCelestiaImpl {
    async fn submit_pay_for_blob(
        &self,
        _fee: Fee,
        _gas_limit: u64,
        blobs: Vec<Blob>,
    ) -> Result<Box<serde_json::value::RawValue>, ErrorObjectOwned> {
        use astria_celestia_jsonrpc_client::state::SubmitPayForBlobResponse;
        use serde_json::{
            to_string,
            value::RawValue,
            Value,
        };
        match &self.mode {
            CelestiaMode::Immediate => {}
            CelestiaMode::DelayedSinceFinalization(delay) => {
                tokio::time::sleep(Duration::from_millis(*delay)).await;
            }
        }

        self.rpc_confirmed_tx.send(blobs).unwrap();

        let rsp = RawValue::from_string(
            to_string(&SubmitPayForBlobResponse {
                height: 100,
                rest: Value::Null,
            })
            .unwrap(),
        )
        .unwrap();

        Ok(rsp)
    }
}

pub struct MockConductor {
    pub bootnode_rx: oneshot::Receiver<Multiaddr>,
    pub block_rx: mpsc::UnboundedReceiver<SequencerBlockData>,
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
        let (block_tx, block_rx) = mpsc::unbounded_channel();
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
            // the "blocks" topic until after the connection is established because otherwise
            // we have to wait until the next gossipsub heartbeat (which is configured to be
            // 10 seconds and runs on a non-tokio timer, i.e. outside its ability to pause and
            // advance time arbitrarily).
            loop {
                let Some(event) = conductor.next().await else {
                    break;
                };
                if let Event::GossipsubPeerConnected(peer_id) = event.unwrap() {
                    debug!(?peer_id, "conductor connected to peer");
                    break;
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

// This is necessary because the Deserialize impl for tendermint::block::Block
// considers the default commit equivalent to being unset.
fn create_non_default_last_commit() -> tendermint::block::Commit {
    use tendermint::block::Commit;
    Commit {
        height: 2u32.into(),
        ..Commit::default()
    }
}

pub(crate) fn create_block_response(
    validator: &Validator,
    height: u32,
    parent: Option<&Response>,
) -> Response {
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

    // for a tendermint block response to convert to a sequencer block data, fields that need to
    // be set are:
    // - last commit
    // - proposer address
    // - height
    // - data hash
    //
    // for submission to celestia (data availability), field that needs to be set:
    // - last block id

    // The first height must not, every height after must contain a commit. constraint set by
    // tendermint for block construction.
    let last_commit = (height != 1).then_some(create_non_default_last_commit());
    // required to convert to sequencer block data (fn
    // convert_block_response_to_sequencer_block_data in ../src/relayer.rs)
    let proposer_address = *validator.address();
    // height greater than current height required to convert to sequencer block data (fn
    // convert_block_response_to_sequencer_block_data in ../src/relayer.rs)
    let height = block::Height::from(height);

    let signing_key = validator.signing_key();

    let suffix = height.to_string().into_bytes();
    let signed_tx = Unsigned::new_with_actions(
        Nonce::from(1),
        vec![Action::new_sequence_action(
            [b"test_chain_id_", &*suffix].concat(),
            [b"hello_world_id_", &*suffix].concat(),
        )],
    )
    .into_signed(signing_key);
    let action_tree_root = [1; 32]; // always goes at index 0 of data
    let data = vec![action_tree_root.to_vec(), signed_tx.to_bytes()];
    // data_hash must be some to convert to sequencer block data (fn from_tendermint_block in
    // ../src/types.rs)
    let data_hash = Some(Hash::Sha256(simple_hash_from_byte_vectors::<sha2::Sha256>(
        &data,
    )));

    let last_block_id = parent.map(|parent| {
        let parent = parent.block.header.hash().as_bytes().to_vec();
        block::Id {
            // block_hash in sequencer block data is set to tendermint block header hash (fn
            // from_tendermint_block in ../src/types.rs)
            hash: Hash::try_from(parent).unwrap(),
            part_set_header: block::parts::Header::default(),
        }
    });

    Response {
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
                height,
                time: Time::now(),
                last_block_id,
                last_commit_hash: None,
                data_hash,
                validators_hash: Hash::Sha256([0; 32]),
                next_validators_hash: Hash::Sha256([0; 32]),
                consensus_hash: Hash::Sha256([0; 32]),
                app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
                last_results_hash: None,
                evidence_hash: None,
                proposer_address,
            },
            data,
            evidence::List::default(),
            last_commit,
        )
        .unwrap(),
    }
}

async fn start_mocked_sequencer() -> MockServer {
    use tendermint::{
        abci,
        hash::AppHash,
    };
    use tendermint_rpc::endpoint::abci_info;
    let server = MockServer::start().await;

    let abci_response = abci_info::Response {
        response: abci::response::Info {
            data: "SequencerRelayerTest".into(),
            version: "1.0.0".into(),
            app_version: 1,
            last_block_height: 5u32.into(),
            last_block_app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
        },
    };
    let abci_response = Wrapper::new_with_id(Id::Num(1), Some(abci_response), None);
    Mock::given(body_partial_json(json!({"method": "abci_info"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(abci_response))
        .mount(&server)
        .await;
    server
}

pub(crate) struct BlockResponseTwoLinkChain {
    pub(crate) parent: Response,
    pub(crate) child: Response,
}

pub(crate) struct BlockResponseFourLinkChain {
    pub(crate) grandparent: Response,
    pub(crate) child: Response,
    pub(crate) parent: Response,
    pub(crate) grandchild: Response,
}

pub(crate) async fn mount_constant_block_response_child_parent_pair(
    sequencer_relayer: &TestSequencerRelayer,
) -> BlockResponseTwoLinkChain {
    let server = &sequencer_relayer.sequencer;
    let validator = &sequencer_relayer.validator;

    // parent response at height 1
    let parent = create_and_mount_block(Duration::ZERO, server, validator, 1, None).await;
    // child response follows at height 2
    let child = create_and_mount_block(Duration::ZERO, server, validator, 2, Some(&parent)).await;

    BlockResponseTwoLinkChain {
        parent,
        child,
    }
}

pub(crate) async fn mount_block(delay: Duration, server: &MockServer, block: Response) {
    let wrapped = Wrapper::new_with_id(Id::Num(1), Some(block), None);
    Mock::given(body_partial_json(json!({"method": "block"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(wrapped)
                .set_delay(delay),
        )
        .up_to_n_times(1) // only listen for one response
        .mount(server)
        .await;
}

pub(crate) async fn create_and_mount_block(
    delay: Duration,
    server: &MockServer,
    validator: &Validator,
    height: u32,
    parent: Option<&Response>,
) -> Response {
    let rsp = create_block_response(validator, height, parent);
    mount_block(delay, server, rsp.clone()).await;
    rsp
}

/// Mounts 4 changing mock responses with the last one repeating
pub(crate) async fn mount_4_changing_block_responses(
    sequencer_relayer: &TestSequencerRelayer,
) -> BlockResponseFourLinkChain {
    let validator = &sequencer_relayer.validator;
    let server = &sequencer_relayer.sequencer;

    let _response_delay = Duration::from_millis(sequencer_relayer.config.block_time);

    // - grandparent is received at 1 tick
    // - parent is received at 4 ticks (delayed 2 ticks)
    // - child is received at 2 ticks (delayed 1 tick)
    // - grandchild is received at 3 ticks

    // each block must be of higher height than current height to convert to sequencer data
    // block. hence height is set to increment with order of arrival.

    let grandparent = create_block_response(validator, 1, None);
    let parent = create_block_response(validator, 4, Some(&grandparent));
    let child = create_block_response(validator, 2, Some(&parent));
    let grandchild = create_block_response(validator, 3, Some(&child));

    // MockServer requires blocks to be mounted in order they should be received
    mount_block(Duration::ZERO, server, grandparent.clone()).await;
    mount_block(Duration::ZERO, server, child.clone()).await;
    mount_block(Duration::ZERO, server, grandchild.clone()).await;
    mount_block(Duration::ZERO, server, parent.clone()).await;

    // grandchild can't finalize
    BlockResponseFourLinkChain {
        grandparent,
        parent,
        child,
        grandchild,
    }
}

pub(crate) async fn mount_two_response_pairs_delayed_children(
    sequencer_relayer: &TestSequencerRelayer,
) -> [BlockResponseTwoLinkChain; 2] {
    let validator = &sequencer_relayer.validator;
    let server = &sequencer_relayer.sequencer;

    // first parent, increase current height to 1 upon arrival
    let parent_one = create_and_mount_block(Duration::ZERO, server, validator, 1, None).await;
    // second parent, increase current height to 2 upon arrival
    let parent_two = create_and_mount_block(Duration::ZERO, server, validator, 2, None).await;

    // delay starts counting from when response is mounted (i.e. from start of test)
    let delay_children = Duration::from_millis(MAX_RELAYER_QUEUE_TIME_MS);
    // the optimistic nature of queued blocks in terms of time out, means the block enqueued
    // before second child, which is the first child, needs to time out parent two. if second
    // child comes delayed but not first child, then finality will be checked before time out is
    // checked for parent two.
    //
    // first child, increase current height to 3 upon arrival
    let child_one =
        create_and_mount_block(delay_children, server, validator, 3, Some(&parent_one)).await;

    // child two needs to arrive after child one to make sure parent two doesn't finalize first
    // (and so the height isn't increased to 4 by child two which would make child one at height 3
    // fail conversion (fn convert_block_response_to_sequencer_block_data in ../src/relayer.rs)
    //
    // second child, increase current height to 4 upon arrival
    let child_two =
        create_and_mount_block(delay_children, server, validator, 4, Some(&parent_two)).await;

    // children can't finalize
    [
        BlockResponseTwoLinkChain {
            parent: parent_one,
            child: child_one,
        },
        BlockResponseTwoLinkChain {
            parent: parent_two,
            child: child_two,
        },
    ]
}
