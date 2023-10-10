use astria_config::AstriaConfig as _;
use astria_sequencer_relayer::{
    telemetry,
    Config,
    SequencerRelayer,
};
use tracing::info;

#[tokio::main]
async fn main() {
    let cfg = Config::get().expect("failed to read configuration");
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
