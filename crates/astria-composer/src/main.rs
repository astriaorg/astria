use astria_composer::{config::get_config, telemetry, Composer};
use tracing::info;

#[tokio::main]
async fn main() {
    let cfg = match get_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("failed to read configuration: {e}");
            std::process::exit(2);
        }
    };
    let cfg_ser = serde_json::to_string(&cfg)
        .expect("the json serializer should never fail when serializing to a string");
    eprintln!("config:\n{cfg_ser}");

    telemetry::init(std::io::stdout, &cfg.log).expect("failed to initialize tracing");

    info!(config = cfg_ser, "initializing composer",);

    Composer::from_config(&cfg)
        .await
        .expect("failed creating composer")
        .run_until_stopped()
        .await;
}
