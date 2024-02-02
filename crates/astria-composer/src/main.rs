use astria_composer::{
    telemetry,
    Composer,
    Config,
};
use tracing::info;

#[tokio::main]
async fn main() {
    let cfg: Config = match config::get() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("failed to read configuration: {e}");
            std::process::exit(2);
        }
    };
    let cfg_ser = serde_json::to_string(&cfg)
        .expect("the json serializer should never fail when serializing to a string");
    eprintln!("config:\n{cfg_ser}");

    let metrics_conf = if cfg.metrics_enabled {
        Some(telemetry::MetricsConfig {
            addr: cfg.prometheus_http_listener_addr,
            labels: Some(vec![("service".into(), "astria-composer".into())]),
            buckets: None,
        })
    } else {
        None
    };

    telemetry::init(std::io::stdout, &cfg.log, metrics_conf).expect("failed to initialize tracing");

    info!(config = cfg_ser, "initializing composer",);

    Composer::from_config(&cfg)
        .expect("failed creating composer")
        .run_until_stopped()
        .await;
}
