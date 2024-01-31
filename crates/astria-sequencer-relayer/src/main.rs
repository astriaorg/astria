use std::net::SocketAddr;

use astria_sequencer_relayer::{
    telemetry,
    Config,
    SequencerRelayer,
};
use metrics_exporter_prometheus::PrometheusBuilder;
use tracing::info;

#[tokio::main]
async fn main() {
    let cfg: Config = config::get().expect("failed to read configuration");
    telemetry::init(std::io::stdout, &cfg.log).expect("failed to setup telemetry");
    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing sequencer relayer"
    );

    if cfg.metrics_enabled {
        let addr: SocketAddr = cfg.prometheus_http_listener_addr.parse().unwrap();

        let builder = PrometheusBuilder::new()
            .with_http_listener(addr)
            .add_global_label("service", "astria_sequencer_relayer");
        builder.install().expect("failed to install recorder/exporter");
    }

    SequencerRelayer::new(&cfg)
        .await
        .expect("could not initialize sequencer relayer")
        .run()
        .await;
}
