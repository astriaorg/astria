use astria_sequencer::{
    sequencer::{Sequencer, DEFAULT_LISTEN_ADDR},
    telemetry,
};

#[tokio::main]
async fn main() {
    telemetry::init(std::io::stdout).expect("failed to initialize telemetry");

    let sequencer = Sequencer::new().await.unwrap();
    sequencer.run(DEFAULT_LISTEN_ADDR).await;
}
