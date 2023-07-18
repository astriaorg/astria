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
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        telemetry::init(std::io::stdout, &filter_directives).unwrap();
    } else {
        telemetry::init(std::io::sink, "").unwrap();
    };
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
    let searcher = searcher::Searcher::new(&config.searcher.clone())
        .await
        .unwrap();

    let searcher_handle = tokio::spawn(searcher.run());

    TestApp {
        config,
        searcher_handle,
    }
}
