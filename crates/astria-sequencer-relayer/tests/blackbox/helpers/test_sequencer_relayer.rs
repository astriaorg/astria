use std::{
    collections::HashSet,
    fmt::{
        self,
        Display,
        Formatter,
    },
    future::Future,
    io::Write,
    mem,
    net::SocketAddr,
    time::Duration,
};

use assert_json_diff::assert_json_include;
use astria_core::primitive::v1::RollupId;
use astria_grpc_mock::MockGuard as GrpcMockGuard;
use astria_sequencer_relayer::{
    config::Config,
    SequencerRelayer,
    ShutdownHandle,
};
use ed25519_consensus::SigningKey;
use futures::TryFutureExt;
use itertools::Itertools;
use once_cell::sync::Lazy;
use reqwest::{
    Response,
    StatusCode,
};
use serde::Deserialize;
use serde_json::json;
use tempfile::NamedTempFile;
use tendermint_config::PrivValidatorKey;
use tendermint_rpc::{
    response::Wrapper,
    Id,
};
use tokio::{
    runtime::{
        self,
        RuntimeFlavor,
    },
    task::{
        yield_now,
        JoinHandle,
    },
};
use tracing::{
    error,
    info,
};
use wiremock::{
    matchers::body_partial_json,
    MockGuard as WireMockGuard,
    MockServer as WireMockServer,
    ResponseTemplate,
};

use super::{
    MockCelestiaAppServer,
    MockSequencerServer,
    SequencerBlockToMount,
};

/// Copied verbatim from
/// [tendermint-rs](https://github.com/informalsystems/tendermint-rs/blob/main/config/tests/support/config/priv_validator_key.ed25519.json)
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

pub struct TestSequencerRelayer {
    /// The socket address that sequencer relayer is serving its API endpoint on
    ///
    /// This is useful for checking if it's healthy, ready, or how many p2p peers
    /// are subscribed to it.
    pub api_address: SocketAddr,

    /// The mocked celestia app server.
    pub celestia_app: MockCelestiaAppServer,

    /// The mocked cometbft server.
    pub cometbft: wiremock::MockServer,

    /// The mocked sequencer server.
    pub sequencer: MockSequencerServer,

    /// A handle which issues a shutdown to the sequencer relayer on being dropped.
    pub relayer_shutdown_handle: Option<ShutdownHandle>,
    pub sequencer_relayer: JoinHandle<()>,

    pub config: Config,

    pub signing_key: SigningKey,

    pub account: tendermint::account::Id,

    pub validator_keyfile: NamedTempFile,

    pub pre_submit_file: NamedTempFile,
    pub post_submit_file: NamedTempFile,
}

impl Drop for TestSequencerRelayer {
    fn drop(&mut self) {
        // We drop the shutdown handle here to cause the sequencer relayer to shut down.
        let _ = self.relayer_shutdown_handle.take();

        let sequencer_relayer = mem::replace(&mut self.sequencer_relayer, tokio::spawn(async {}));
        let _ = futures::executor::block_on(async move {
            tokio::time::timeout(Duration::from_secs(2), sequencer_relayer)
                .await
                .unwrap_or_else(|_| {
                    error!("timed out waiting for sequencer relayer to shut down");
                    Ok(())
                })
        });
    }
}

impl TestSequencerRelayer {
    /// Mounts a `CometBFT` ABCI Info response.
    pub async fn mount_abci_response(&self, height: u32) -> WireMockGuard {
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
        wiremock::Mock::given(body_partial_json(json!({"method": "abci_info"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(abci_response))
            .up_to_n_times(1)
            .expect(1..)
            .named("CometBFT abci_info")
            .mount_as_scoped(&self.cometbft)
            .await
    }

    /// Mounts a Sequencer block response.
    ///
    /// The `debug_name` is assigned to the mock and is output on error to assist with debugging.
    pub async fn mount_sequencer_block_response<const RELAY_SELF: bool>(
        &self,
        block_to_mount: SequencerBlockToMount,
        debug_name: impl Into<String>,
    ) -> GrpcMockGuard {
        self.sequencer
            .mount_sequencer_block_response::<RELAY_SELF>(self.account, block_to_mount, debug_name)
            .await
    }

    /// Mounts a Celestia `BroadcastTx` response.
    ///
    /// The `debug_name` is assigned to the mock and is output on error to assist with debugging.
    /// It is also assigned as the `TxHash` in the response.
    pub async fn mount_celestia_app_broadcast_tx_response(
        &self,
        debug_name: impl Into<String>,
    ) -> GrpcMockGuard {
        self.celestia_app
            .mount_broadcast_tx_response(debug_name)
            .await
    }

    /// Mounts a Celestia `GetTx` response.
    ///
    /// The `debug_name` is assigned to the mock and is output on error to assist with debugging.
    /// It is also assigned as the `TxHash` in the request and response.
    pub async fn mount_celestia_app_get_tx_response(
        &self,
        celestia_height: i64,
        debug_name: impl Into<String>,
    ) -> GrpcMockGuard {
        self.celestia_app
            .mount_get_tx_response(celestia_height, debug_name)
            .await
    }

    /// Gets the state reported via the "status" endpoint of the sequencer-relayer.
    ///
    /// # Panics
    ///
    /// Panics if the state cannot be retrieved within 100 milliseconds.
    pub async fn state(&self, context: &str) -> SequencerRelayerState {
        let (value, _status_code) = self.get_from_relayer_api("status", context).await;
        serde_json::from_value(value).expect("should parse status")
    }

    /// Gets the status reported via the "healthz" endpoint of the sequencer-relayer.
    ///
    /// # Panics
    ///
    /// Panics if the status cannot be retrieved within 100 milliseconds.
    pub async fn healthz(&self, context: &str) -> (String, StatusCode) {
        let (value, status_code) = self.get_from_relayer_api("healthz", context).await;
        let zpage: ZPage = serde_json::from_value(value).expect("should parse healthz");
        (zpage.status, status_code)
    }

    /// Gets the status reported via the "readyz" endpoint of the sequencer-relayer.
    ///
    /// # Panics
    ///
    /// Panics if the status cannot be retrieved within 100 milliseconds.
    pub async fn readyz(&self, context: &str) -> (String, StatusCode) {
        let (value, status_code) = self.get_from_relayer_api("readyz", context).await;
        let zpage: ZPage = serde_json::from_value(value).expect("should parse readyz");
        (zpage.status, status_code)
    }

    /// Polls the "status" endpoint of the sequencer-relayer until it reports
    /// `latest_confirmed_celestia_height` equal to `height`.
    ///
    /// # Panics
    ///
    /// Panics if any individual request exceeds 100 milliseconds, cannot be parsed, or if the
    /// endpoint doesn't report the given height within the number of milliseconds specified.
    pub async fn wait_for_latest_confirmed_celestia_height(&self, height: u64, within_ms: u64) {
        let predicate = |value: serde_json::Value, status_code: StatusCode| -> bool {
            if !status_code.is_success() {
                return false;
            }
            let state: SequencerRelayerState =
                serde_json::from_value(value).expect("should parse status");
            state.latest_confirmed_celestia_height == Some(height)
        };

        let context = "waiting for latest confirmed celestia height";
        self.wait_until_relayer_api_matches("status", within_ms, context, predicate)
            .await;
    }

    /// Polls the "status" endpoint of the sequencer-relayer until it reports
    /// `latest_fetched_sequencer_height` equal to `height`.
    ///
    /// # Panics
    ///
    /// Panics if any individual request exceeds 100 milliseconds, cannot be parsed, or if the
    /// endpoint doesn't report the given height within the number of milliseconds specified.
    pub async fn wait_for_latest_fetched_sequencer_height(&self, height: u64, within_ms: u64) {
        let predicate = |value: serde_json::Value, status_code: StatusCode| -> bool {
            if !status_code.is_success() {
                return false;
            }
            let state: SequencerRelayerState =
                serde_json::from_value(value).expect("should parse status");
            state.latest_fetched_sequencer_height == Some(height)
        };

        let context = "waiting for latest fetched sequencer height";
        self.wait_until_relayer_api_matches("status", within_ms, context, predicate)
            .await;
    }

    /// Polls the "status" endpoint of the sequencer-relayer until it reports
    /// `latest_observed_sequencer_height` equal to `height`.
    ///
    /// # Panics
    ///
    /// Panics if any individual request exceeds 100 milliseconds, cannot be parsed, or if the
    /// endpoint doesn't report the given height within the number of milliseconds specified.
    pub async fn wait_for_latest_observed_sequencer_height(&self, height: u64, within_ms: u64) {
        let predicate = |value: serde_json::Value, status_code: StatusCode| -> bool {
            if !status_code.is_success() {
                return false;
            }
            let state: SequencerRelayerState =
                serde_json::from_value(value).expect("should parse status");
            state.latest_observed_sequencer_height == Some(height)
        };

        let context = "waiting for latest observed sequencer height";
        self.wait_until_relayer_api_matches("status", within_ms, context, predicate)
            .await;
    }

    /// Polls the "healthz" endpoint of the sequencer-relayer until it responds with the given
    /// status code.  Returns the value of the response's `status` field.
    ///
    /// # Panics
    ///
    /// Panics if any individual request exceeds 100 milliseconds, cannot be parsed, or if the
    /// endpoint doesn't respond with the given code within the number of milliseconds specified.
    pub async fn wait_for_healthz(
        &self,
        code: StatusCode,
        within_ms: u64,
        context: &str,
    ) -> String {
        self.wait_for_zpage(ZPageType::Healthz, code, within_ms, context)
            .await
    }

    /// Polls the "readyz" endpoint of the sequencer-relayer until it responds with the given
    /// status code.  Returns the value of the response's `status` field.
    ///
    /// # Panics
    ///
    /// Panics if any individual request exceeds 100 milliseconds, cannot be parsed, or if the
    /// endpoint doesn't respond with the given code within the number of milliseconds specified.
    pub async fn wait_for_readyz(&self, code: StatusCode, within_ms: u64, context: &str) -> String {
        self.wait_for_zpage(ZPageType::Readyz, code, within_ms, context)
            .await
    }

    /// Polls `is_finished` on the sequencer-relayer task until it returns `true`.
    ///
    /// # Panics
    ///
    /// Panics if the relayer task doesn't finish within the number of milliseconds specified.
    pub async fn wait_for_relayer_shutdown(&mut self, within_ms: u64) {
        let relayer_join_handle = mem::replace(&mut self.sequencer_relayer, tokio::spawn(async {}));
        let check_finished = async {
            loop {
                if relayer_join_handle.is_finished() {
                    return;
                }
                yield_now().await;
            }
        };
        self.timeout_ms(
            within_ms,
            "waiting for sequencer-relayer to finish",
            check_finished,
        )
        .await;
    }

    pub fn celestia_app_received_blob_count(&self) -> usize {
        self.celestia_app.namespaces.lock().unwrap().len()
    }

    pub fn has_celestia_app_received_blob_from_rollup(&self, rollup_id: RollupId) -> bool {
        let namespace = astria_core::celestia::namespace_v0_from_rollup_id(rollup_id);
        self.celestia_app
            .namespaces
            .lock()
            .unwrap()
            .iter()
            .contains(&namespace)
    }

    /// Polls the given z-page endpoint of the sequencer-relayer until it responds with the given
    /// status code.  Returns the value of the response's `status` field.
    ///
    /// # Panics
    ///
    /// Panics if any individual request exceeds 100 milliseconds, cannot be parsed, or if the
    /// endpoint doesn't respond with the given code within the number of milliseconds specified.
    async fn wait_for_zpage(
        &self,
        zpage: ZPageType,
        code: StatusCode,
        within_ms: u64,
        context: &str,
    ) -> String {
        let predicate = |_, status_code: StatusCode| -> bool { status_code == code };
        let (value, _status_code) = self
            .wait_until_relayer_api_matches(
                zpage.to_string().as_str(),
                within_ms,
                context,
                predicate,
            )
            .await;
        let zpage: ZPage = serde_json::from_value(value).expect("should parse {zpage}");
        zpage.status
    }

    /// Polls the given endpoint of the sequencer-relayer until it responds with data satisfying
    /// `predicate`.
    ///
    /// The predicate is passed the JSON value and status code from the response, and if it returns
    /// `true`, the function returns.
    ///
    /// # Panics
    ///
    /// Panics if any individual request exceeds 100 milliseconds, cannot be parsed, or if the
    /// predicate isn't satisfied within the number of milliseconds specified.
    async fn wait_until_relayer_api_matches<P>(
        &self,
        api_endpoint: &str,
        within_ms: u64,
        context: &str,
        mut predicate: P,
    ) -> (serde_json::Value, StatusCode)
    where
        P: FnMut(serde_json::Value, StatusCode) -> bool,
    {
        let getter = async {
            loop {
                let (value, status_code) = self.get_from_relayer_api(api_endpoint, context).await;
                if predicate(value.clone(), status_code) {
                    return (value, status_code);
                }
                yield_now().await;
            }
        };
        self.timeout_ms(within_ms, context, getter).await
    }

    /// Gets the JSON body reported via the given endpoint of the sequencer-relayer.
    ///
    /// # Panics
    ///
    /// Panics if the response cannot be retrieved within 100 milliseconds.
    async fn get_from_relayer_api(
        &self,
        api_endpoint: &str,
        context: &str,
    ) -> (serde_json::Value, StatusCode) {
        let url = format!("http://{}/{api_endpoint}", self.api_address);
        let getter = async {
            reqwest::get(&url)
                .await
                .unwrap_or_else(|error| panic!("should get response from `{url}`: {error}"))
        };

        let new_context = format!("{context}: get from `{url}`");
        let response = self.timeout_ms(100, &new_context, getter).await;

        let status_code = response.status();
        let value = response
            .json::<serde_json::Value>()
            .await
            .unwrap_or_else(|error| {
                panic!("{context}: failed to parse response from `{url}` as JSON: {error}")
            });
        (value, status_code)
    }

    /// Executes `future` within the specified duration, returning its result.
    ///
    /// If execution takes more than 80% of the allowed time, an error is logged before returning.
    ///
    /// # Panics
    ///
    /// Panics if execution takes longer than the specified duration.
    pub async fn timeout_ms<F: Future>(
        &self,
        num_milliseconds: u64,
        context: &str,
        future: F,
    ) -> F::Output {
        let start = std::time::Instant::now();
        let within = Duration::from_millis(num_milliseconds);
        if let Ok(value) = tokio::time::timeout(within, future).await {
            let elapsed = start.elapsed();
            if elapsed * 5 > within * 4 {
                error!(%context,
                    "elapsed time ({} seconds) was over 80% of the specified timeout ({} \
                     seconds) - consider increasing the timeout",
                    elapsed.as_secs_f32(),
                    within.as_secs_f32()
                );
            }
            value
        } else {
            let state = tokio::time::timeout(Duration::from_millis(100), async {
                reqwest::get(format!("http://{}/status", self.api_address))
                    .and_then(Response::json)
                    .await
                    .ok()
            })
            .await
            .unwrap_or(None)
            .and_then(|value: serde_json::Value| {
                serde_json::from_value::<SequencerRelayerState>(value).ok()
            })
            .map_or("unknown".to_string(), |state| format!("{state:?}"));

            let healthz = tokio::time::timeout(Duration::from_millis(100), async {
                reqwest::get(format!("http://{}/healthz", self.api_address))
                    .and_then(Response::json)
                    .await
                    .ok()
            })
            .await
            .unwrap_or(None)
            .and_then(|value: serde_json::Value| serde_json::from_value::<ZPage>(value).ok())
            .map_or("unknown".to_string(), |zpage| zpage.status);

            panic!("timed out; context: `{context}`, state: `{state}`, healthz: `{healthz}`");
        }
    }

    #[track_caller]
    pub fn assert_state_files_are_as_expected(
        &self,
        pre_sequencer_height: u32,
        post_sequencer_height: u32,
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

// allow: want the name to reflect this is a test config.
#[allow(clippy::module_name_repetitions)]
pub struct TestSequencerRelayerConfig {
    /// Sets up the test relayer to ignore all blocks except those proposed by the same address
    /// stored in its validator key.
    pub relay_only_self: bool,
    /// Sets the start height of relayer and configures the on-disk pre- and post-submit files to
    /// look accordingly.
    pub last_written_sequencer_height: Option<u64>,
    /// The rollup ID filter, to be stringified and provided as `Config::only_include_rollups`
    /// value.
    pub only_include_rollups: HashSet<RollupId>,
}

impl TestSequencerRelayerConfig {
    pub async fn spawn_relayer(self) -> TestSequencerRelayer {
        assert_ne!(
            runtime::Handle::current().runtime_flavor(),
            RuntimeFlavor::CurrentThread,
            "the sequencer relayer must be run on a multi-threaded runtime, e.g. the test could \
             be configured using `#[tokio::test(flavor = \"multi_thread\", worker_threads = 1)]`"
        );
        Lazy::force(&TELEMETRY);

        let celestia_app = MockCelestiaAppServer::spawn().await;
        let celestia_app_grpc_endpoint = format!("http://{}", celestia_app.local_addr);
        let celestia_keyfile = write_file(
            b"c8076374e2a4a58db1c924e3dafc055e9685481054fe99e58ed67f5c6ed80e62".as_slice(),
        )
        .await;

        let validator_keyfile = write_file(PRIVATE_VALIDATOR_KEY.as_bytes()).await;
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

        let cometbft = WireMockServer::start().await;

        let sequencer = MockSequencerServer::spawn().await;
        let sequencer_grpc_endpoint = format!("http://{}", sequencer.local_addr);

        let (pre_submit_file, post_submit_file) =
            if let Some(last_written_sequencer_height) = self.last_written_sequencer_height {
                create_files_for_start_at_height(last_written_sequencer_height)
            } else {
                create_files_for_fresh_start()
            };

        let only_include_rollups = self.only_include_rollups.iter().join(",").to_string();

        let config = Config {
            cometbft_endpoint: cometbft.uri(),
            sequencer_grpc_endpoint,
            celestia_app_grpc_endpoint,
            celestia_app_key_file: celestia_keyfile.path().to_string_lossy().to_string(),
            block_time: 1000,
            relay_only_validator_key_blocks: self.relay_only_self,
            validator_key_file: validator_keyfile.path().to_string_lossy().to_string(),
            only_include_rollups,
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
        let (sequencer_relayer, relayer_shutdown_handle) =
            SequencerRelayer::new(config.clone()).unwrap();
        let api_address = sequencer_relayer.local_addr();
        let sequencer_relayer = tokio::task::spawn(sequencer_relayer.run());

        TestSequencerRelayer {
            api_address,
            celestia_app,
            config,
            sequencer,
            cometbft,
            relayer_shutdown_handle: Some(relayer_shutdown_handle),
            sequencer_relayer,
            signing_key,
            account: address,
            validator_keyfile,
            pre_submit_file,
            post_submit_file,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct SequencerRelayerState {
    pub ready: bool,
    pub celestia_connected: bool,
    pub sequencer_connected: bool,
    pub latest_confirmed_celestia_height: Option<u64>,
    pub latest_fetched_sequencer_height: Option<u64>,
    pub latest_observed_sequencer_height: Option<u64>,
    pub latest_requested_sequencer_height: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct ZPage {
    status: String,
}

enum ZPageType {
    Healthz,
    Readyz,
}

impl Display for ZPageType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ZPageType::Healthz => formatter.write_str("healthz"),
            ZPageType::Readyz => formatter.write_str("readyz"),
        }
    }
}

async fn write_file(data: &'static [u8]) -> NamedTempFile {
    tokio::task::spawn_blocking(|| {
        let keyfile = NamedTempFile::new().unwrap();
        (&keyfile).write_all(data).unwrap();
        keyfile
    })
    .await
    .unwrap()
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
