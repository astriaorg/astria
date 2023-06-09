use astria_sequencer::{
    sequencer::Sequencer,
    telemetry,
};

/// The default address to listen on; this corresponds to the default ABCI
/// application address expected by tendermint.
pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:26658";

#[tokio::main]
async fn main() {
    telemetry::init(std::io::stdout).expect("failed to initialize telemetry");
    Sequencer::run_until_stopped(DEFAULT_LISTEN_ADDR)
        .await
        .expect("failed to run sequencer");
}
