use std::net::SocketAddr;

use assert_json_diff::assert_json_include;
use astria_core::sequencer::v1alpha1::{
    test_utils::ConfigureCometBftBlock,
    RollupId,
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
    astria_eyre::install().unwrap();
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        println!("initializing telemetry");
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::stdout)
            .force_stdout()
            .pretty_print()
            .filter_directives(&filter_directives)
            .try_init()
            .unwrap();
    } else {
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::sink)
            .try_init()
            .unwrap();
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

    pub pre_submit_file: NamedTempFile,
    pub post_submit_file: NamedTempFile,
}

impl TestSequencerRelayer {
    pub async fn mount_abci_response(&self, height: u32) -> MockGuard {
        use tendermint::{
            abci,
            hash::AppHash,
        };
        use tendermint_rpc::endpoint::abci_info;
        let abci_response = abci_info::Response {
            response: abci::response::Info {
                data: "SequencerRelayerTest".into(),
                version: "1.0.0".into(),
                app_version: 1,
                last_block_height: height.into(),
                last_block_app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
            },
        };
        let abci_response = Wrapper::new_with_id(Id::Num(1), Some(abci_response), None);
        Mock::given(body_partial_json(json!({"method": "abci_info"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(abci_response))
            .up_to_n_times(1)
            .expect(1..)
            .mount_as_scoped(&self.sequencer)
            .await
    }

    pub async fn mount_block_response<const RELAY_SELF: bool>(&self, height: u32) -> MockGuard {
        let proposer = if RELAY_SELF {
            self.account
        } else {
            tendermint::account::Id::try_from(vec![0u8; 20]).unwrap()
        };
        let block_response = create_block_response(&self.signing_key, proposer, height);
        let wrapped = Wrapper::new_with_id(Id::Num(1), Some(block_response.clone()), None);
        let matcher = body_partial_json(json!({
            "method": "block",
            "params": {
                "height": format!("{height}")
            }
        }));
        Mock::given(matcher)
            .respond_with(ResponseTemplate::new(200).set_body_json(wrapped))
            .expect(1)
            .mount_as_scoped(&self.sequencer)
            .await
    }

    pub async fn mount_bad_block_response(&self, height: u32) -> MockGuard {
        let matcher = body_partial_json(json!({
            "method": "block",
            "params": {
                "height": format!("{height}")
            }
        }));
        Mock::given(matcher)
            .respond_with(ResponseTemplate::new(500))
            .expect(1..)
            .mount_as_scoped(&self.sequencer)
            .await
    }

    #[track_caller]
    pub fn assert_state_files_are_as_expected(
        &self,
        pre_sequencer_height: u64,
        post_sequencer_height: u64,
    ) {
        let pre_submit_state: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&self.config.pre_submit_path).unwrap())
                .unwrap();
        assert_json_include!(
            actual: pre_submit_state,
            expected: json!({
                "sequencer_height": pre_sequencer_height
            }),
        );

        let post_submit_state: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&self.config.post_submit_path).unwrap())
                .unwrap();
        assert_json_include!(
            actual: post_submit_state,
            expected: json!({
                "sequencer_height": post_sequencer_height,
            }),
        );
    }
}

pub struct TestSequencerRelayerConfig {
    // Sets up the test relayer to ignore all blocks except those proposed by the same address
    // stored in its validator key.
    pub relay_only_self: bool,
    // Sets the start height of relayer and configures the on-disk pre- and post-submit files to
    // look accordingly.
    pub last_written_sequencer_height: Option<u64>,
}

impl TestSequencerRelayerConfig {
    pub async fn spawn_relayer(self) -> TestSequencerRelayer {
        Lazy::force(&TELEMETRY);

        let mut celestia = MockCelestia::start().await;
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

        let sequencer = MockServer::start().await;

        let (pre_submit_file, post_submit_file) =
            if let Some(last_written_sequencer_height) = self.last_written_sequencer_height {
                create_files_for_start_at_height(last_written_sequencer_height)
            } else {
                create_files_for_fresh_start()
            };

        let config = Config {
            sequencer_endpoint: sequencer.uri(),
            celestia_endpoint: format!("http://{celestia_addr}"),
            celestia_bearer_token: String::new(),
            block_time: 1000,
            relay_only_validator_key_blocks: self.relay_only_self,
            validator_key_file: Some(keyfile.path().to_string_lossy().to_string()),
            api_addr: "0.0.0.0:0".into(),
            log: String::new(),
            force_stdout: false,
            no_otel: false,
            no_metrics: false,
            metrics_http_listener_addr: String::new(),
            pretty_print: true,
            pre_submit_path: pre_submit_file.path().to_owned(),
            post_submit_path: post_submit_file.path().to_owned(),
        };

        info!(config = serde_json::to_string(&config).unwrap());
        let config_clone = config.clone();
        let sequencer_relayer = SequencerRelayer::new(&config_clone).await.unwrap();
        let api_address = sequencer_relayer.local_addr();
        let sequencer_relayer = tokio::task::spawn(sequencer_relayer.run());

        TestSequencerRelayer {
            api_address,
            celestia,
            config,
            sequencer,
            sequencer_relayer,
            signing_key,
            account: address,
            keyfile,
            pre_submit_file,
            post_submit_file,
        }
    }
}

fn create_files_for_fresh_start() -> (NamedTempFile, NamedTempFile) {
    let pre = NamedTempFile::new()
        .expect("must be able to create an empty pre submit state file to run tests");
    let post = NamedTempFile::new()
        .expect("must be able to create an empty post submit state file to run tests");
    serde_json::to_writer(
        &pre,
        &json!({
            "state": "ignore"
        }),
    )
    .expect("must be able to write pre-submit state to run tests");
    serde_json::to_writer(
        &post,
        &json!({
            "state": "fresh"
        }),
    )
    .expect("must be able to write post-submit state to run tests");
    (pre, post)
}

fn create_files_for_start_at_height(height: u64) -> (NamedTempFile, NamedTempFile) {
    let pre = NamedTempFile::new()
        .expect("must be able to create an empty pre submit state file to run tests");
    let post = NamedTempFile::new()
        .expect("must be able to create an empty post submit state file to run tests");

    serde_json::to_writer(
        &pre,
        &json!({
            "state": "ignore",
        }),
    )
    .expect("must be able to write pre state to file to run tests");
    serde_json::to_writer_pretty(
        &post,
        &json!({
            "state": "submitted",
            "celestia_height": 5,
            "sequencer_height": height
        }),
    )
    .expect("must be able to write post state to file to run tests");
    (pre, post)
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
    async fn start() -> Self {
        use jsonrpsee::server::ServerBuilder;
        let (addr_tx, addr_rx) = oneshot::channel();
        let server = ServerBuilder::default().build("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        addr_tx.send(addr).unwrap();
        let (state_rpc_confirmed_tx, state_rpc_confirmed_rx) = mpsc::unbounded_channel();
        let state_celestia = BlobServerImpl {
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
        Ok(100)
    }
}

fn create_block_response(
    signing_key: &SigningKey,
    proposer_address: tendermint::account::Id,
    height: u32,
) -> endpoint::block::Response {
    use tendermint::{
        block,
        Hash,
    };
    let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_name_1");
    let data = b"hello_world_id_1".to_vec();
    let block = ConfigureCometBftBlock {
        height,
        signing_key: Some(signing_key.clone()),
        proposer_address: Some(proposer_address),
        rollup_transactions: vec![(rollup_id, data)],
    }
    .make();

    endpoint::block::Response {
        block_id: block::Id {
            hash: Hash::Sha256([0; 32]),
            part_set_header: block::parts::Header::new(0, Hash::None).unwrap(),
        },
        block,
    }
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
