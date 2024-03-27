use astria_conductor::{
    Conductor,
    Config,
};
use once_cell::sync::Lazy;

mod mock_grpc;
pub use mock_grpc::MockGrpc;
use tokio::task::JoinHandle;
use wiremock::MockServer;

const CELESTIA_BEARER_TOKEN: &str = "ABCDEFGH";

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

pub struct TestConductor {
    pub mock_grpc: MockGrpc,
    pub mock_http: MockServer,
    pub conductor: JoinHandle<()>,
}

pub async fn spawn_conductor() -> TestConductor {
    Lazy::force(&TELEMETRY);

    let mock_grpc = MockGrpc::spawn().await;
    let mock_http = MockServer::start().await;

    let config = Config {
        celestia_node_http_url: mock_http.uri(),
        execution_rpc_url: format!("http://{}", mock_grpc.local_addr),
        sequencer_cometbft_url: mock_http.uri(),
        sequencer_grpc_url: format!("http://{}", mock_grpc.local_addr),
        ..make_config()
    };

    let conductor = {
        let conductor = Conductor::new(config).await.unwrap();
        tokio::spawn(conductor.run_until_stopped())
    };

    TestConductor {
        conductor,
        mock_grpc,
        mock_http,
    }
}

fn make_config() -> Config {
    Config {
        celestia_block_time_ms: 12000,
        celestia_node_http_url: "http://127.0.0.1:26658".into(),
        celestia_bearer_token: CELESTIA_BEARER_TOKEN.into(),
        sequencer_grpc_url: "http://127.0.0.1:8080".into(),
        sequencer_cometbft_url: "http://127.0.0.1:26657".into(),
        sequencer_block_time_ms: 2000,
        execution_rpc_url: "http://127.0.0.1:50051".into(),
        log: "info".into(),
        execution_commit_level: astria_conductor::config::CommitLevel::SoftAndFirm,
        force_stdout: false,
        no_otel: false,
        no_metrics: true,
        metrics_http_listener_addr: "".into(),
        pretty_print: false,
    }
}
