use astria_composer::{
    config::{
        self,
        Config,
    },
    searcher::{self,},
    telemetry,
};
use once_cell::sync::Lazy;
use tokio::task::JoinHandle;

static TRACING: Lazy<()> = Lazy::new(|| {
    let res = if let Some(_) = std::env::var_os("TEST_LOG") {
        // if TEST_LOG is set, use stdout for tracing at the level specified by RUST_LOG
        let log = std::env::var_os("RUST_LOG").unwrap().into_string().unwrap();
        telemetry::init(&log, std::io::stdout)
    } else {
        telemetry::init(&"info", std::io::sink)
    };
    if res.is_err() {
        eprintln!("failed setting up telemetry for tests: {res:?}");
    }
});

pub fn init_env() {
    Lazy::force(&TRACING);
    // TODO: init env and return a TestEnvironment
}

pub struct TestApp {
    pub config: Config,
    pub searcher_handle: JoinHandle<()>,
}

pub async fn spawn_app() -> TestApp {
    init_env();
    let config = config::Config::default();
    let searcher = searcher::Searcher::new(&config.searcher.clone()).unwrap();

    let searcher_handle = tokio::spawn(searcher.run());

    TestApp {
        config,
        searcher_handle,
    }
}
