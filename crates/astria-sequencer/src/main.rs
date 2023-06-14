use astria_sequencer::{
    telemetry,
    Config,
    Sequencer,
};
use tracing::info;

/// The default address to listen on; this corresponds to the default ABCI
/// application address expected by tendermint.
pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:26658";

#[tokio::main]
async fn main() {
    let config = Config::get();
    telemetry::init(std::io::stdout).expect("failed to initialize telemetry");
    info!(
        config = serde_json::to_string(&config).unwrap(),
        "starting sequencer"
    );
    Sequencer::run_until_stopped(config)
        .await
        .expect("failed to run sequencer");
}
