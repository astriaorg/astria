// TODO: tracing

use astria_composer::{
    config::{
        self,
        Config,
    },
    searcher::{self,},
    telemetry,
};
use once_cell::sync::Lazy;

static TRACING: Lazy<()> = Lazy::new(|| {
    let res = if std::env::var_os("TEST_LOG").is_some() {
        telemetry::init(std::io::stdout)
    } else {
        telemetry::init(std::io::sink)
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
}

pub async fn spawn_app() -> TestApp {
    init_env();
    let config = config::get().unwrap();
    let searcher = searcher::Searcher::new(&config.searcher.clone()).unwrap();

    let _ = tokio::spawn(searcher.run());

    TestApp {
        config,
    }
}
