use astria_composer::{
    config,
    telemetry,
    Composer,
};
use color_eyre::eyre;
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cfg = config::get().expect("failed to read configuration");
    let cfg_ser = serde_json::to_string(&cfg)
        .expect("the json serializer should never fail when serializing to a string");
    eprintln!("config:\n{cfg_ser}");

    telemetry::init(std::io::stdout, &cfg.log).expect("failed to initialize tracing");

    info!(config = cfg_ser, "initializing composer",);

    let _composer = Composer::from_config(&cfg)
        .await
        .expect("failed creating composer")
        .run_until_stopped()
        .await;

    Ok(())
}
