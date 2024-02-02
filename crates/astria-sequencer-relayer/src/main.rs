use astria_sequencer_relayer::{
    telemetry,
    Config,
    SequencerRelayer,
};
use tracing::info;

#[tokio::main]
async fn main() {
    let cfg: Config = config::get().expect("failed to read configuration");
    let metrics_addr = if cfg.metrics_enabled {
        Some(cfg.prometheus_http_listener_addr)
    } else {
        None
    };

    telemetry::init(std::io::stdout, &cfg.log, metrics_addr).expect("failed to setup telemetry");
    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing sequencer relayer"
    );

    SequencerRelayer::new(&cfg)
        .await
        .expect("could not initialize sequencer relayer")
        .run()
        .await;
}
