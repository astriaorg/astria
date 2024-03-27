use std::{
    collections::HashMap,
    net::SocketAddr,
    time::Duration,
};

use astria_composer::{
    config::Config,
    Composer,
};
use astria_eyre::eyre;
use once_cell::sync::Lazy;
use test_utils::mock::Geth;
use tokio::task::JoinHandle;
use wiremock::MockGuard;

pub mod mock_sequencer;

static TELEMETRY: Lazy<()> = Lazy::new(|| {
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::stdout)
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

pub struct TestComposer {
    pub cfg: Config,
    pub composer: JoinHandle<eyre::Result<()>>,
    pub rollup_nodes: HashMap<String, Geth>,
    pub sequencer: wiremock::MockServer,
    pub setup_guard: MockGuard,
    pub grpc_collector_addr: SocketAddr,
}

/// Spawns composer in a test environment.
///
/// # Panics
/// There is no explicit error handling in favour of panicking loudly
/// and early.
pub async fn spawn_composer(rollup_ids: &[&str]) -> TestComposer {
    Lazy::force(&TELEMETRY);

    let mut rollup_nodes = HashMap::new();
    let mut rollups = String::new();
    for id in rollup_ids {
        let geth = Geth::spawn().await;
        let execution_url = format!("ws://{}", geth.local_addr());
        rollup_nodes.insert((*id).to_string(), geth);
        rollups.push_str(&format!("{id}::{execution_url},"));
    }
    let (sequencer, sequencer_setup_guard) = mock_sequencer::start().await;
    let sequencer_url = sequencer.uri();
    let config = Config {
        log: String::new(),
        api_listen_addr: "127.0.0.1:0".parse().unwrap(),
        rollups,
        sequencer_url,
        private_key: "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90"
            .to_string()
            .into(),
        block_time_ms: 2000,
        max_bytes_per_bundle: 200_000,
        no_otel: false,
        force_stdout: false,
        no_metrics: true,
        metrics_http_listener_addr: String::new(),
        pretty_print: true,
        grpc_addr: "127.0.0.1:0".parse().unwrap(),
    };
    let (composer_addr, grpc_collector_addr, composer_handle) = {
        let composer = Composer::from_config(&config).await.unwrap();
        let composer_addr = composer.local_addr();
        let grpc_collector_addr = composer.grpc_collector_local_addr().unwrap();
        let task = tokio::spawn(composer.run_until_stopped());
        (composer_addr, grpc_collector_addr, task)
    };

    loop_until_composer_is_ready(composer_addr).await;
    TestComposer {
        cfg: config,
        composer: composer_handle,
        rollup_nodes,
        sequencer,
        setup_guard: sequencer_setup_guard,
        grpc_collector_addr,
    }
}

/// Query composer's `/readyz` endpoint until its ready.
///
/// # Panics
///
/// Panics instead of handling errors if no HTTP request could be sent to
/// composer or if its response could not be deserialized as JSON.
pub async fn loop_until_composer_is_ready(addr: SocketAddr) {
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
        tokio::time::sleep(Duration::from_secs(1)).await;
        if readyz.status.to_lowercase() == "ok" {
            break;
        }
    }
}
