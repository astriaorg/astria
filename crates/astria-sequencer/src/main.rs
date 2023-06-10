use astria_sequencer::{
    sequencer::{
        Sequencer,
        DEFAULT_LISTEN_ADDR,
    },
    telemetry,
};

#[tokio::main]
async fn main() {
    telemetry::init(std::io::stdout).expect("failed to initialize telemetry");
    Sequencer::run_until_stopped(DEFAULT_LISTEN_ADDR)
        .await
        .expect("failed to run sequencer");
}
