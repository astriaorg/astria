use astria_sequencer_relayer::{
    metrics_init,
    telemetry,
    Config,
    SequencerRelayer,
};
use tracing::info;

#[tokio::main]
async fn main() {
    let cfg: Config = config::get().expect("failed to read configuration");

    let metrics_conf = if cfg.metrics_enabled {
        Some(telemetry::MetricsConfig {
            addr: cfg.prometheus_http_listener_addr.clone(),
            service: "astria-sequencer-relayer",
            buckets: Some(metrics_init::HISTOGRAM_BUCKETS.to_vec()),
        })
    } else {
        None
    };
    metrics_init::register();
    telemetry::init(std::io::stdout, &cfg.log, metrics_conf).expect("failed to setup telemetry");
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
