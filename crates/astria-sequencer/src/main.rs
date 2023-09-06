use astria_sequencer::{
    config,
    Sequencer,
};
use tracing::info;

#[tokio::main]
async fn main() {
    let config = match config::get() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("failed to read configuration: {e}");
            std::process::exit(2);
        }
    };
    telemetry::init(std::io::stdout, &config.log).expect("failed to initialize telemetry");
    info!(
        config = serde_json::to_string(&config).unwrap(),
        "starting sequencer"
    );
    Sequencer::run_until_stopped(config)
        .await
        .expect("failed to run sequencer");
}
