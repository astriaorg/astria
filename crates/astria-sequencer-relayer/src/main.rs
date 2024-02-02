use astria_sequencer_relayer::{
    metrics,
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
            addr: cfg.prometheus_http_listener_addr,
            labels: Some(vec![("service".into(), "astria-sequencer-relayer".into())]),
            buckets: Some(metrics::HISTOGRAM_BUCKETS.to_vec()),
        })
    } else {
        None
    };
    metrics::register();
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
