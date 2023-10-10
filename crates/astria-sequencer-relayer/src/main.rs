use astria_sequencer_relayer::{
    config::get_config,
    telemetry,
    SequencerRelayer,
};
use tracing::info;

#[tokio::main]
async fn main() {
    let cfg = get_config().expect("failed to read configuration");
    telemetry::init(std::io::stdout, &cfg.log).expect("failed to setup telemetry");
    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing sequencer relayer"
    );

    SequencerRelayer::new(cfg)
        .expect("could not initialize sequencer relayer")
        .run()
        .await;
}
