use astria_sequencer::{
    Config,
    Sequencer,
};
use tracing::info;

#[tokio::main]
async fn main() {
    let config = Config::get();
    telemetry::init(std::io::stdout, config.log.as_deref().unwrap_or("info"))
        .expect("failed to initialize telemetry");
    info!(
        config = serde_json::to_string(&config).unwrap(),
        "starting sequencer"
    );
    Sequencer::run_until_stopped(config)
        .await
        .expect("failed to run sequencer");
}
