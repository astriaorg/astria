use std::{
    net::SocketAddr,
    time::Duration,
};

use astria_sequencer_relayer::{
    config::Config,
    telemetry,
    SequencerRelayer,
};
use celestia_client::celestia_types::{
    blob::SubmitOptions,
    Blob,
};
use ed25519_consensus::SigningKey;
use once_cell::sync::Lazy;
use proto::native::sequencer::v1alpha1::{
    asset::default_native_asset_id,
    RollupId,
    SequenceAction,
    UnsignedTransaction,
};
use serde_json::json;
use tempfile::NamedTempFile;
use tendermint_config::PrivValidatorKey;
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

    pub signing_key: SigningKey,
    pub account: tendermint::account::Id,

    pub keyfile: NamedTempFile,
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
        let block_response = create_block_response(&self.signing_key, self.account, 1);
        let wrapped = Wrapper::new_with_id(Id::Num(1), Some(block_response.clone()), None);
        Mock::given(body_partial_json(json!({"method": "block"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(wrapped))
            .expect(1)
            .up_to_n_times(1)
            .mount_as_scoped(&self.sequencer)
            .await
    }

    pub async fn mount_block_response(&self, height: u32) -> MockGuard {
        let block_response = create_block_response(&self.signing_key, self.account, height);
        let wrapped = Wrapper::new_with_id(Id::Num(1), Some(block_response.clone()), None);
        Mock::given(body_partial_json(json!({"method": "block"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(wrapped))
            .expect(1)
            .mount_as_scoped(&self.sequencer)
            .await
    }

    pub async fn mount_block_response_with_zero_proposer(&self, height: u32) -> MockGuard {
        let proposer = tendermint::account::Id::try_from(vec![0u8; 20]).unwrap();
        let block_response = create_block_response(&self.signing_key, proposer, height);
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

pub async fn spawn_sequencer_relayer_relay_all(
    celestia_mode: CelestiaMode,
) -> TestSequencerRelayer {
    spawn_sequencer_relayer(celestia_mode, false).await
}

pub async fn spawn_sequencer_relayer(
    celestia_mode: CelestiaMode,
    relay_only_validator_key_blocks: bool,
) -> TestSequencerRelayer {
    Lazy::force(&TELEMETRY);
    let block_time = 1000;

    let mut celestia = MockCelestia::start(block_time, celestia_mode).await;
    let celestia_addr = (&mut celestia.addr_rx).await.unwrap();

    let keyfile = tokio::task::spawn_blocking(|| {
        use std::io::Write as _;

        let keyfile = NamedTempFile::new().unwrap();
        (&keyfile)
            .write_all(PRIVATE_VALIDATOR_KEY.as_bytes())
            .unwrap();
        keyfile
    })
    .await
    .unwrap();
    let PrivValidatorKey {
        address,
        priv_key,
        ..
    } = PrivValidatorKey::parse_json(PRIVATE_VALIDATOR_KEY).unwrap();
    let signing_key = priv_key
        .ed25519_signing_key()
        .cloned()
        .unwrap()
        .try_into()
        .unwrap();

    let sequencer = start_mocked_sequencer().await;

    let config = Config {
        sequencer_endpoint: sequencer.uri(),
        celestia_endpoint: format!("http://{celestia_addr}"),
        celestia_bearer_token: String::new(),
        block_time: 1000,
        relay_only_validator_key_blocks,
        validator_key_file: Some(keyfile.path().to_string_lossy().to_string()),
        rpc_port: 0,
        log: String::new(),
    };

    info!(config = serde_json::to_string(&config).unwrap());
    let config_clone = config.clone();
    let sequencer_relayer = SequencerRelayer::new(&config_clone).await.unwrap();
    let api_address = sequencer_relayer.local_addr();
    let sequencer_relayer = tokio::task::spawn(sequencer_relayer.run());

    loop_until_sequencer_relayer_is_ready(api_address).await;

    TestSequencerRelayer {
        api_address,
        celestia,
        config,
        sequencer,
        sequencer_relayer,
        signing_key,
        account: address,
        keyfile,
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

use celestia_mock::{
    BlobServer,
    HeaderServer,
};
use jsonrpsee::{
    core::async_trait,
    server::ServerHandle,
    types::ErrorObjectOwned,
};

pub struct MockCelestia {
    pub addr_rx: oneshot::Receiver<SocketAddr>,
    pub state_rpc_confirmed_rx: mpsc::UnboundedReceiver<Vec<Blob>>,
    pub server_handle: ServerHandle,
}

impl MockCelestia {
    async fn start(sequencer_block_time_ms: u64, mode: CelestiaMode) -> Self {
        use jsonrpsee::server::ServerBuilder;
        let (addr_tx, addr_rx) = oneshot::channel();
        let server = ServerBuilder::default().build("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        addr_tx.send(addr).unwrap();
        let (state_rpc_confirmed_tx, state_rpc_confirmed_rx) = mpsc::unbounded_channel();
        let state_celestia = BlobServerImpl {
            sequencer_block_time_ms,
            mode,
            rpc_confirmed_tx: state_rpc_confirmed_tx,
        };
        let header_celestia = HeaderServerImpl;
        let mut merged_celestia = state_celestia.into_rpc();
        merged_celestia.merge(header_celestia.into_rpc()).unwrap();
        let server_handle = server.start(merged_celestia);
        Self {
            addr_rx,
            state_rpc_confirmed_rx,
            server_handle,
        }
    }
}

struct HeaderServerImpl;

#[async_trait]
impl HeaderServer for HeaderServerImpl {
    async fn header_network_head(
        &self,
    ) -> Result<celestia_client::celestia_types::ExtendedHeader, ErrorObjectOwned> {
        use celestia_client::{
            celestia_tendermint::{
                block::{
                    header::Header,
                    Commit,
                },
                validator,
            },
            celestia_types::{
                DataAvailabilityHeader,
                ExtendedHeader,
            },
        };
        let header = ExtendedHeader {
            header: Header {
                height: 42u32.into(),
                ..make_celestia_tendermint_header()
            },
            commit: Commit {
                height: 42u32.into(),
                ..Commit::default()
            },
            validator_set: validator::Set::without_proposer(vec![]),
            dah: DataAvailabilityHeader {
                row_roots: vec![],
                column_roots: vec![],
            },
        };
        Ok(header)
    }
}

struct BlobServerImpl {
    sequencer_block_time_ms: u64,
    mode: CelestiaMode,
    rpc_confirmed_tx: mpsc::UnboundedSender<Vec<Blob>>,
}

#[async_trait]
impl BlobServer for BlobServerImpl {
    async fn blob_submit(
        &self,
        blobs: Vec<Blob>,
        _opts: SubmitOptions,
    ) -> Result<u64, ErrorObjectOwned> {
        self.rpc_confirmed_tx.send(blobs).unwrap();
        if let CelestiaMode::Delayed(n) = self.mode {
            tokio::time::sleep(Duration::from_millis(n * self.sequencer_block_time_ms)).await;
        }
        Ok(100)
    }
}

fn create_block_response(
    signing_key: &SigningKey,
    proposer_address: tendermint::account::Id,
    height: u32,
) -> endpoint::block::Response {
    use proto::Message as _;
    use sha2::Digest as _;
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
    let suffix = height.to_string().into_bytes();
    let rollup_id = RollupId::from_unhashed_bytes([b"test_chain_id_", &*suffix].concat());
    let signed_tx = UnsignedTransaction {
        nonce: 1,
        actions: vec![
            SequenceAction {
                rollup_id,
                data: [b"hello_world_id_", &*suffix].concat(),
            }
            .into(),
        ],
        fee_asset_id: default_native_asset_id(),
    }
    .into_signed(signing_key);
    let rollup_txs = proto::native::sequencer::v1alpha1::merge_sequence_actions_in_signed_transaction_transactions_by_rollup_id(
        &[signed_tx.clone()]
    );
    let action_tree_root =
        proto::native::sequencer::v1alpha1::derive_merkle_tree_from_rollup_txs(&rollup_txs).root();

    let chain_ids_commitment = merkle::Tree::from_leaves(std::iter::once(rollup_id)).root();
    let data = vec![
        action_tree_root.to_vec(),
        chain_ids_commitment.to_vec(),
        signed_tx.into_raw().encode_to_vec(),
    ];
    let data_hash = Some(Hash::Sha256(simple_hash_from_byte_vectors::<sha2::Sha256>(
        &data.iter().map(sha2::Sha256::digest).collect::<Vec<_>>(),
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
                proposer_address,
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
        signing_key: &SigningKey,
        account: tendermint::account::Id,
        height: u32,
    ) -> endpoint::block::Response {
        let rsp = create_block_response(signing_key, account, height);
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
    let signing_key = &sequencer_relayer.signing_key;
    let account = sequencer_relayer.account;
    let server = &sequencer_relayer.sequencer;

    let mut rsps = Vec::new();
    // The first one resolves immediately
    rsps.push(create_and_mount_block(Duration::ZERO, server, signing_key, account, 1).await);

    for i in 2..=3 {
        rsps.push(create_and_mount_block(response_delay, server, signing_key, account, i).await);
    }

    // The last one will repeat
    rsps.push(create_block_response(signing_key, account, 4));
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

#[allow(clippy::missing_panics_doc)]
#[must_use]
/// Returns a default tendermint block header for test purposes.
pub fn make_celestia_tendermint_header() -> celestia_client::celestia_tendermint::block::Header {
    use celestia_client::celestia_tendermint::{
        account,
        block::{
            header::Version,
            Header,
            Height,
        },
        chain,
        hash::AppHash,
        Hash,
        Time,
    };

    Header {
        version: Version {
            block: 0,
            app: 0,
        },
        chain_id: chain::Id::try_from("test").unwrap(),
        height: Height::from(1u32),
        time: Time::now(),
        last_block_id: None,
        last_commit_hash: Hash::None,
        data_hash: Hash::None,
        validators_hash: Hash::Sha256([0; 32]),
        next_validators_hash: Hash::Sha256([0; 32]),
        consensus_hash: Hash::Sha256([0; 32]),
        app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
        last_results_hash: Hash::None,
        evidence_hash: Hash::None,
        proposer_address: account::Id::try_from([0u8; 20].to_vec()).unwrap(),
    }
}
