use astria_bridge_withdrawer::BridgeWithdrawer;
use astria_eyre::eyre;
use once_cell::sync::Lazy;
use tokio::task::JoinHandle;
use wiremock::MockServer;

const SEQUENCER_CHAIN_ID: &str = "test_sequencer-1000";
const DEFAULT_LAST_ROLLUP_HEIGHT: u64 = 1;
const DEFAULT_IBC_DENOM: &str = "transfer/channel-0/utia";

static TELEMETRY: Lazy<()> = Lazy::new(|| {
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::stdout)
            .set_pretty_print(true)
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

struct TestBridgeWithdrawer {
    bridge_withdrawer: Option<BridgeWithdrawer>,
    cometbft_mock: MockServer,
    bridge_withdrawer_task_handle: Option<JoinHandle<Result<(), eyre::Report>>>,
}

impl TestBridgeWithdrawer {
    async fn setup() -> Self {
        Lazy::force(&TELEMETRY);

        // set up external resources
        let shutdown_token = CancellationToken::new();

        // sequencer signer key
        let keyfile = NamedTempFile::new().unwrap();
        (&keyfile)
            .write_all(
                "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90".as_bytes(),
            )
            .unwrap();
        let sequencer_key_path = keyfile.path().to_str().unwrap().to_string();

        // cometbft
        let cometbft_mock = MockServer::start().await;
        let sequencer_cometbft_endpoint = format!("http://{}", cometbft_mock.address());
        // TODO: use grpc mock
        let sequencer_grpc_endpoint = format!("http://{}", cometbft_mock.address());

        // withdrawer state
        let state = Arc::new(state::State::new());
        // not testing watcher here so just set it to ready
        state.set_watcher_ready();
        let (startup_tx, startup_rx) = oneshot::channel();
        let startup_handle = startup::SubmitterHandle::new(startup_rx);

        let metrics = Box::leak(Box::new(Metrics::new()));

        let (submitter, submitter_handle) = submitter::Builder {
            shutdown_token: shutdown_token.clone(),
            startup_handle,
            sequencer_key_path,
            sequencer_address_prefix: "astria".into(),
            sequencer_cometbft_endpoint,
            sequencer_grpc_endpoint,
            state,
            metrics,
        }
        .build()
        .unwrap();

        Self {
            submitter: Some(submitter),
            bridge_withdrawer_task_handle: None,
            startup_tx: Some(startup_tx),
            submitter_handle,
            cometbft_mock,
        }
    }

    async fn startup(&mut self) {
        let submitter = self.submitter.take().unwrap();

        let mut state = submitter.state.subscribe();

        self.bridge_withdrawer_task_handle = Some(tokio::spawn(submitter.run()));

        self.startup_tx
            .take()
            .expect("should only send startup info once")
            .send(startup::SubmitterInfo {
                sequencer_chain_id: SEQUENCER_CHAIN_ID.to_string(),
            })
            .unwrap();

        // wait for the submitter to be ready
        state
            .wait_for(state::StateSnapshot::is_ready)
            .await
            .unwrap();
    }

    async fn spawn() -> Self {
        let mut submitter = Self::setup().await;
        submitter.startup().await;
        submitter
    }
}
