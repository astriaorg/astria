use std::{
    net::SocketAddr,
    time::Duration,
};

use astria_composer::{
    config::Config,
    telemetry,
    Composer,
};
use once_cell::sync::Lazy;
use tokio::task::JoinHandle;
use tracing::debug;
pub mod mock_geth;
pub mod mock_sequencer;

static TELEMETRY: Lazy<()> = Lazy::new(|| {
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        telemetry::init(std::io::stdout, &filter_directives).unwrap();
    } else {
        telemetry::init(std::io::sink, "").unwrap();
    }
});

pub struct TestComposer {
    pub composer: JoinHandle<()>,
    pub geth: mock_geth::MockGeth,
    pub sequencer: wiremock::MockServer,
}

/// Spawns composer in a test environment.
///
/// # Panics
/// There is no explicit error handling in favour of panicking loudly
/// and early.
pub async fn spawn_composer() -> TestComposer {
    Lazy::force(&TELEMETRY);

    let geth = mock_geth::MockGeth::spawn().await;
    let execution_url = format!("ws://{}", geth.local_addr());
    let sequencer = mock_sequencer::start().await;
    let sequencer_url = sequencer.uri();
    let config = Config {
        log: String::new(),
        api_listen_addr: "127.0.0.1:0".parse().unwrap(),
        chain_id: "testtest".into(),
        sequencer_url,
        execution_url,
    };
    let (composer_addr, composer) = {
        let composer = Composer::from_config(&config).await.unwrap();
        let composer_addr = composer.local_addr();
        let task = tokio::spawn(composer.run_until_stopped());
        (composer_addr, task)
    };

    debug!("looping until composer is ready");
    loop_until_composer_is_ready(composer_addr).await;
    TestComposer {
        composer,
        geth,
        sequencer,
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
