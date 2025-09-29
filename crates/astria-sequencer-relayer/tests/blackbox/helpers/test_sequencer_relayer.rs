use std::{
    collections::HashSet,
    fmt::{
        self,
        Display,
        Formatter,
    },
    fs,
    future::Future,
    io::Write,
    mem,
    net::SocketAddr,
    sync::LazyLock,
    time::Duration,
};

use assert_json_diff::assert_json_include;
use astria_core::{
    crypto::SigningKey,
    primitive::v1::RollupId,
};
use astria_grpc_mock::MockGuard as GrpcMockGuard;
use astria_sequencer_relayer::{
    config::Config,
    Metrics,
    SequencerRelayer,
    ShutdownHandle,
};
use http::StatusCode;
use itertools::Itertools as _;
use serde::Deserialize;
use serde_json::json;
use telemetry::metrics;
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
    MockServer as WireMockServer,
    ResponseTemplate,
};

use super::{
    MockCelestiaAppServer,
    MockSequencerServer,
    SequencerBlockToMount,
};

const SEQUENCER_CHAIN_ID: &str = "test-sequencer";
const CELESTIA_CHAIN_ID: &str = "test-celestia";

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

const STATUS_RESPONSE: &str = r#"
{
  "node_info": {
    "protocol_version": {
      "p2p": "8",
      "block": "11",
      "app": "0"
    },
    "id": "a1d3bbddb7800c6da2e64169fec281494e963ba3",
    "listen_addr": "tcp://0.0.0.0:26656",
    "network": "test",
    "version": "0.38.6",
    "channels": "40202122233038606100",
    "moniker": "fullnode",
    "other": {
      "tx_index": "on",
      "rpc_address": "tcp://0.0.0.0:26657"
    }
  },
  "sync_info": {
    "latest_block_hash": "A4202E4E367712AC2A797860265A7EBEA8A3ACE513CB0105C2C9058449641202",
    "latest_app_hash": "BCC9C9B82A49EC37AADA41D32B4FBECD2441563703955413195BDA2236775A68",
    "latest_block_height": "452605",
    "latest_block_time": "2024-05-09T15:59:17.849713071Z",
    "earliest_block_hash": "C34B7B0B82423554B844F444044D7D08A026D6E413E6F72848DB2F8C77ACE165",
    "earliest_app_hash": "6B776065775471CEF46AC75DE09A4B869A0E0EB1D7725A04A342C0E46C16F472",
    "earliest_block_height": "1",
    "earliest_block_time": "2024-04-23T00:49:11.964127Z",
    "catching_up": false
  },
  "validator_info": {
    "address": "0B46F33BA2FA5C2E2AD4C4C4E5ECE3F1CA03D195",
    "pub_key": {
      "type": "tendermint/PubKeyEd25519",
      "value": "bA6GipHUijVuiYhv+4XymdePBsn8EeTqjGqNQrBGZ4I="
    },
    "voting_power": "0"
  }
}"#;

static TELEMETRY: LazyLock<()> = LazyLock::new(|| {
    astria_eyre::install().unwrap();
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "astria_sequencer_relayer=trace,blackbox=trace,info".into());
        println!("initializing telemetry");
        let _ = telemetry::configure()
            .set_no_otel(true)
            .set_force_stdout(true)
            .set_filter_directives(&filter_directives)
            .try_init::<Metrics>(&())
            .unwrap();
    } else {
        let _ = telemetry::configure()
            .set_no_otel(true)
            .set_stdout_writer(std::io::sink)
            .try_init::<Metrics>(&())
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

    pub submission_state_file: NamedTempFile,
    /// The sequencer chain ID which will be returned by the mock `cometbft` instance, and set via
    /// `TestSequencerRelayerConfig`.
    pub actual_sequencer_chain_id: String,
    /// The Celestia chain ID which will be returned by the mock `celestia_app` instance, and set
    /// via `TestSequencerRelayerConfig`.
    pub actual_celestia_chain_id: String,
    pub metrics_handle: metrics::Handle,
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
    pub async fn mount_abci_response(&self, height: u32) {
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
            .mount(&self.cometbft)
            .await;
    }

    /// Mounts a Sequencer block response.
    ///
    /// The `debug_name` is assigned to the mock and is output on error to assist with debugging.
    pub async fn mount_sequencer_block_response(
        &self,
        block_to_mount: SequencerBlockToMount,
        debug_name: impl Into<String>,
    ) {
        self.sequencer
            .mount_sequencer_block_response(block_to_mount, debug_name)
            .await;
    }

    /// Mounts a Sequencer block response and returns a `GrpcMockGuard` to allow for waiting for
    /// the mock to be satisfied.
    ///
    /// The `debug_name` is assigned to the mock and is output on error to assist with debugging.
    pub async fn mount_sequencer_block_response_as_scoped(
        &self,
        block_to_mount: SequencerBlockToMount,
        debug_name: impl Into<String>,
    ) -> GrpcMockGuard {
        self.sequencer
            .mount_sequencer_block_response_as_scoped(block_to_mount, debug_name)
            .await
    }

    /// Mounts a Celestia `BroadcastTx` response.
    ///
    /// The `debug_name` is assigned to the mock and is output on error to assist with debugging.
    /// It is also assigned as the `TxHash` in the response.
    pub async fn mount_celestia_app_broadcast_tx_response(&self, debug_name: impl Into<String>) {
        self.celestia_app
            .mount_broadcast_tx_response(debug_name)
            .await;
    }

    /// Mounts a Celestia `BroadcastTx` response and returns a `GrpcMockGuard` to allow for waiting
    /// for the mock to be satisfied.
    ///
    /// The `debug_name` is assigned to the mock and is output on error to assist with debugging.
    /// It is also assigned as the `TxHash` in the response.
    pub async fn mount_celestia_app_broadcast_tx_response_as_scoped(
        &self,
        debug_name: impl Into<String>,
    ) -> GrpcMockGuard {
        self.celestia_app
            .mount_broadcast_tx_response_as_scoped(debug_name)
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
    ) {
        self.celestia_app
            .mount_get_tx_response(celestia_height, debug_name)
            .await;
    }

    /// Mounts a Celestia `GetTx` response and returns a `GrpcMockGuard` to allow for waiting for
    /// the mock to be satisfied.
    ///
    /// The `debug_name` is assigned to the mock and is output on error to assist with debugging.
    /// It is also assigned as the `TxHash` in the request and response.
    pub async fn mount_celestia_app_get_tx_response_as_scoped(
        &self,
        celestia_height: i64,
        debug_name: impl Into<String>,
    ) -> GrpcMockGuard {
        self.celestia_app
            .mount_get_tx_response_as_scoped(celestia_height, debug_name)
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
            reqwest::get(url.clone())
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
            if elapsed.checked_mul(5).unwrap() > within.checked_mul(4).unwrap() {
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
                    .await
                    .ok()?
                    .json()
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
                    .await
                    .ok()?
                    .json()
                    .await
                    .ok()
            })
            .await
            .unwrap_or(None)
            .and_then(|value: serde_json::Value| serde_json::from_value::<ZPage>(value).ok())
            .map_or("unknown".to_string(), |zpage| zpage.status);

            error!("timed out; context: `{context}`, state: `{state}`, healthz: `{healthz}`");
            panic!("timed out; context: `{context}`, state: `{state}`, healthz: `{healthz}`");
        }
    }

    #[track_caller]
    pub fn check_state_file(&self, last_sequencer_height: u32, current_sequencer_height: u32) {
        let submission_state: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&self.config.submission_state_path).unwrap())
                .unwrap();
        assert_json_include!(
            actual: submission_state,
            expected: json!({ "sequencer_height": last_sequencer_height }),
        );

        assert_json_include!(
            actual: submission_state,
            expected: json!({ "sequencer_height": current_sequencer_height }),
        );
    }

    /// Mounts a `CometBFT` status response with the chain ID set as per
    /// `TestSequencerRelayerConfig::sequencer_chain_id`.
    async fn mount_cometbft_status_response(&self) {
        use tendermint_rpc::endpoint::status;

        let mut status_response: status::Response = serde_json::from_str(STATUS_RESPONSE).unwrap();
        status_response.node_info.network = self.actual_sequencer_chain_id.parse().unwrap();

        let response = Wrapper::new_with_id(Id::Num(1), Some(status_response), None);
        wiremock::Mock::given(body_partial_json(json!({"method": "status"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .up_to_n_times(1)
            .expect(1..)
            .named("CometBFT status")
            .mount(&self.cometbft)
            .await;
    }
}

pub struct TestSequencerRelayerConfig {
    /// Sets the start height of relayer and configures the on-disk submission-state file to
    /// look accordingly.
    pub last_written_sequencer_height: Option<u64>,
    /// The rollup ID filter, to be stringified and provided as `Config::only_include_rollups`
    /// value.
    pub only_include_rollups: HashSet<RollupId>,
    /// The sequencer chain ID.
    pub sequencer_chain_id: String,
    /// The Celestia chain ID.
    pub celestia_chain_id: String,
}

impl TestSequencerRelayerConfig {
    pub async fn spawn_relayer(self) -> TestSequencerRelayer {
        assert_ne!(
            runtime::Handle::current().runtime_flavor(),
            RuntimeFlavor::CurrentThread,
            "the sequencer relayer must be run on a multi-threaded runtime, e.g. the test could \
             be configured using `#[tokio::test(flavor = \"multi_thread\", worker_threads = 1)]`"
        );
        LazyLock::force(&TELEMETRY);

        let celestia_app = MockCelestiaAppServer::spawn(self.celestia_chain_id.clone()).await;
        let celestia_app_grpc_endpoint = format!("http://{}", celestia_app.local_addr);
        let celestia_keyfile = write_file(
            b"c8076374e2a4a58db1c924e3dafc055e9685481054fe99e58ed67f5c6ed80e62".as_slice(),
        )
        .await;

        let PrivValidatorKey {
            priv_key, ..
        } = PrivValidatorKey::parse_json(PRIVATE_VALIDATOR_KEY).unwrap();
        let signing_key = priv_key
            .ed25519_signing_key()
            .cloned()
            .unwrap()
            .as_bytes()
            .try_into()
            .unwrap();

        let cometbft = WireMockServer::start().await;

        let sequencer = MockSequencerServer::spawn().await;
        let sequencer_grpc_endpoint = format!("http://{}", sequencer.local_addr);

        let submission_state_file =
            if let Some(last_written_sequencer_height) = self.last_written_sequencer_height {
                create_file_for_start_at_height(last_written_sequencer_height)
            } else {
                create_file_for_fresh_start()
            };

        let only_include_rollups = self.only_include_rollups.iter().join(",").to_string();

        let config = Config {
            sequencer_chain_id: SEQUENCER_CHAIN_ID.to_string(),
            celestia_chain_id: CELESTIA_CHAIN_ID.to_string(),
            cometbft_endpoint: cometbft.uri(),
            sequencer_grpc_endpoint,
            celestia_app_grpc_endpoint,
            celestia_app_key_file: celestia_keyfile.path().to_string_lossy().to_string(),
            block_time: 1000,
            only_include_rollups,
            api_addr: "0.0.0.0:0".into(),
            log: String::new(),
            force_stdout: false,
            no_otel: false,
            no_metrics: false,
            metrics_http_listener_addr: "127.0.0.1:9000".to_string(),
            submission_state_path: submission_state_file.path().to_owned(),
            celestia_default_min_gas_price: 0.002,
        };

        let (metrics, metrics_handle) = metrics::ConfigBuilder::new()
            .set_global_recorder(false)
            .build(&())
            .unwrap();
        let metrics = Box::leak(Box::new(metrics));

        info!(config = serde_json::to_string(&config).unwrap());
        let (sequencer_relayer, relayer_shutdown_handle) =
            SequencerRelayer::new(config.clone(), metrics)
                .await
                .unwrap();
        let api_address = sequencer_relayer.local_addr();
        let sequencer_relayer = tokio::task::spawn(sequencer_relayer.run());

        let test_sequencer_relayer = TestSequencerRelayer {
            api_address,
            celestia_app,
            config,
            sequencer,
            cometbft,
            relayer_shutdown_handle: Some(relayer_shutdown_handle),
            sequencer_relayer,
            signing_key,
            submission_state_file,
            actual_sequencer_chain_id: self.sequencer_chain_id,
            actual_celestia_chain_id: self.celestia_chain_id,
            metrics_handle,
        };

        test_sequencer_relayer
            .mount_cometbft_status_response()
            .await;

        test_sequencer_relayer
    }
}

impl Default for TestSequencerRelayerConfig {
    fn default() -> Self {
        Self {
            last_written_sequencer_height: None,
            only_include_rollups: HashSet::new(),
            sequencer_chain_id: SEQUENCER_CHAIN_ID.to_string(),
            celestia_chain_id: CELESTIA_CHAIN_ID.to_string(),
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

fn create_file_for_fresh_start() -> NamedTempFile {
    let temp_file = NamedTempFile::new()
        .expect("must be able to create an empty submission state file to run tests");
    serde_json::to_writer(&temp_file, &json!({ "state": "fresh" }))
        .expect("must be able to write submission state to run tests");
    temp_file
}

fn create_file_for_start_at_height(height: u64) -> NamedTempFile {
    let temp_file = NamedTempFile::new()
        .expect("must be able to create an empty submission state file to run tests");
    serde_json::to_writer_pretty(
        &temp_file,
        &json!({
            "state": "started",
            "sequencer_height": height.saturating_add(10),
            "last_submission": {
                "celestia_height": 5,
                "sequencer_height": height
            }
        }),
    )
    .expect("must be able to write submission state to run tests");
    temp_file
}
