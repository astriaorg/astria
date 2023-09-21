use std::{
    net::SocketAddr,
    time::Duration,
};

use astria_sequencer_relayer::{
    config::Config,
    telemetry,
    validator::Validator,
    SequencerRelayer,
};
use astria_sequencer_validation::MerkleTree;
use once_cell::sync::Lazy;
use proto::native::sequencer::v1alpha1::{
    SequenceAction,
    UnsignedTransaction,
};
use serde_json::json;
use tempfile::NamedTempFile;
use tendermint_rpc::{
    endpoint,
    response::Wrapper,
    Id,
};
use tokio::{
    sync::{
        mpsc,
        oneshot,
    },
    task::JoinHandle,
    time,
};
use tracing::info;
use wiremock::{
    matchers::body_partial_json,
    Mock,
    MockGuard,
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

    /// The mocked sequencer service (also serving as tendermint jsonrpc?)
    pub sequencer: MockServer,

    pub sequencer_relayer: JoinHandle<()>,

    pub config: Config,

    pub validator: Validator,

    pub _keyfile: NamedTempFile,
}

impl TestSequencerRelayer {
    pub async fn advance_by_block_time(&self) {
        time::advance(Duration::from_millis(self.config.block_time + 100)).await;
    }

    // Mount a block response on the mocks erver inside the test sequencer relayer env.
    //
    // Returns a MockGuard that can be used to verify that a block was picked up
    // by sequencer-relayer.
    pub async fn mount_initial_block_response(&self) -> MockGuard {
        let block_response = create_block_response(&self.validator, 1);
        let wrapped = Wrapper::new_with_id(Id::Num(1), Some(block_response.clone()), None);
        Mock::given(body_partial_json(json!({"method": "block"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(wrapped))
            .expect(1)
            .up_to_n_times(1)
            .mount_as_scoped(&self.sequencer)
            .await
    }

    pub async fn mount_block_response(&self, height: u32) -> MockGuard {
        let block_response = create_block_response(&self.validator, height);
        let wrapped = Wrapper::new_with_id(Id::Num(1), Some(block_response.clone()), None);
        Mock::given(body_partial_json(json!({"method": "block"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(wrapped))
            .expect(1)
            .mount_as_scoped(&self.sequencer)
            .await
    }
}

pub enum CelestiaMode {
    Immediate,
    Delayed(u64),
}

pub async fn spawn_sequencer_relayer(celestia_mode: CelestiaMode) -> TestSequencerRelayer {
    Lazy::force(&TELEMETRY);
    let block_time = 1000;

    let mut celestia = MockCelestia::start(block_time, celestia_mode).await;
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

    let config = Config {
        sequencer_endpoint: sequencer.uri(),
        celestia_endpoint: format!("http://{celestia_addr}"),
        celestia_bearer_token: "".into(),
        gas_limit: 100000,
        block_time: 1000,
        validator_key_file: keyfile.path().to_string_lossy().to_string(),
        rpc_port: 0,
        log: "".into(),
    };

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
    async fn start(sequencer_block_time_ms: u64, mode: CelestiaMode) -> Self {
        use jsonrpsee::server::ServerBuilder;
        let (addr_tx, addr_rx) = oneshot::channel();
        let server = ServerBuilder::default().build("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        addr_tx.send(addr).unwrap();
        let (state_rpc_confirmed_tx, state_rpc_confirmed_rx) = mpsc::unbounded_channel();
        let state_celestia = StateCelestiaImpl {
            sequencer_block_time_ms,
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
    sequencer_block_time_ms: u64,
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

        self.rpc_confirmed_tx.send(blobs).unwrap();

        let rsp = RawValue::from_string(
            to_string(&SubmitPayForBlobResponse {
                height: 100,
                rest: Value::Null,
            })
            .unwrap(),
        )
        .unwrap();
        if let CelestiaMode::Delayed(n) = self.mode {
            tokio::time::sleep(Duration::from_millis(n * self.sequencer_block_time_ms)).await;
        }

        Ok(rsp)
    }
}

fn create_block_response(validator: &Validator, height: u32) -> endpoint::block::Response {
    use proto::Message as _;
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

    let suffix = height.to_string().into_bytes();
    let chain_id = [b"test_chain_id_", &*suffix].concat();
    let signed_tx_bytes = UnsignedTransaction {
        nonce: 1,
        actions: vec![
            SequenceAction {
                chain_id: chain_id.clone(),
                data: [b"hello_world_id_", &*suffix].concat(),
            }
            .into(),
        ],
    }
    .into_signed(signing_key)
    .into_raw()
    .encode_to_vec();
    let action_tree =
        astria_sequencer_validation::MerkleTree::from_leaves(vec![signed_tx_bytes.clone()]);
    let chain_ids_commitment = MerkleTree::from_leaves(vec![chain_id]).root();
    let data = vec![
        action_tree.root().to_vec(),
        chain_ids_commitment.to_vec(),
        signed_tx_bytes,
    ];
    let data_hash = Some(Hash::Sha256(simple_hash_from_byte_vectors::<sha2::Sha256>(
        &data,
    )));

    let (last_commit_hash, last_commit) = sequencer_types::test_utils::make_test_commit_and_hash();

    endpoint::block::Response {
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
                height: block::Height::from(height),
                time: Time::now(),
                last_block_id: None,
                last_commit_hash: (height > 1).then_some(last_commit_hash),
                data_hash,
                validators_hash: Hash::Sha256([0; 32]),
                next_validators_hash: Hash::Sha256([0; 32]),
                consensus_hash: Hash::Sha256([0; 32]),
                app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
                last_results_hash: None,
                evidence_hash: None,
                proposer_address: *validator.address(),
            },
            data,
            evidence::List::default(),
            // The first height must not, every height after must contain a last commit
            (height > 1).then_some(last_commit),
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

/// Mounts 4 changing mock responses with the last one repeating
pub async fn mount_4_changing_block_responses(
    sequencer_relayer: &TestSequencerRelayer,
) -> Vec<endpoint::block::Response> {
    async fn create_and_mount_block(
        delay: Duration,
        server: &MockServer,
        validator: &Validator,
        height: u32,
    ) -> endpoint::block::Response {
        let rsp = create_block_response(validator, height);
        let wrapped = Wrapper::new_with_id(Id::Num(1), Some(rsp.clone()), None);
        Mock::given(body_partial_json(json!({"method": "block"})))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(wrapped)
                    .set_delay(delay),
            )
            .up_to_n_times(1)
            .mount(server)
            .await;
        rsp
    }

    let response_delay = Duration::from_millis(sequencer_relayer.config.block_time);
    let validator = &sequencer_relayer.validator;
    let server = &sequencer_relayer.sequencer;

    let mut rsps = Vec::new();
    // The first one resolves immediately
    rsps.push(create_and_mount_block(Duration::ZERO, server, validator, 1).await);

    for i in 2..=3 {
        rsps.push(create_and_mount_block(response_delay, server, validator, i).await);
    }

    // The last one will repeat
    rsps.push(create_block_response(validator, 4));
    let wrapped = Wrapper::new_with_id(Id::Num(1), Some(rsps[3].clone()), None);
    Mock::given(body_partial_json(json!({"method": "block"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(wrapped)
                .set_delay(response_delay),
        )
        .mount(&sequencer_relayer.sequencer)
        .await;
    rsps
}
