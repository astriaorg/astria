use astria_conductor::telemetry;
use astria_conductor_test::TestEnvironment;
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

pub async fn init_test() -> TestEnvironment {
    Lazy::force(&TRACING);
    astria_conductor_test::init_test().await
}
