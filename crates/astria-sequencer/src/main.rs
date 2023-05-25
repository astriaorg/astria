use astria_sequencer::sequencer::Sequencer;
use tracing_subscriber::EnvFilter;

/// The default address to listen on; this corresponds to the default ABCI
/// application address expected by tendermint.
pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:26658";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let sequencer = Sequencer::new().await.unwrap();
    sequencer.run(DEFAULT_LISTEN_ADDR).await;
}
