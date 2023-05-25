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

    let sequencer = Sequencer::new().await.unwrap();
    sequencer.run(DEFAULT_LISTEN_ADDR).await;
}
