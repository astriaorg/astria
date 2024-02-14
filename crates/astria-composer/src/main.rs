use std::process::ExitCode;

use astria_composer::{
    telemetry,
    Composer,
    Config,
    BUILD_INFO,
};
use color_eyre::eyre::WrapErr as _;
use tracing::info;

#[tokio::main]
async fn main() -> ExitCode {
    eprintln!(
        "{}",
        serde_json::to_string(&BUILD_INFO)
            .expect("build info is serializable because it contains only unicode fields")
    );
    let cfg: Config = match config::get() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("failed to read configuration: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut telemetry_conf = telemetry::configure()
        .set_no_otel(cfg.no_otel)
        .set_force_stdout(cfg.force_stdout)
        .filter_directives(&cfg.log);

    if !cfg.no_metrics {
        telemetry_conf = telemetry_conf
            .metrics_addr(&cfg.metrics_http_listener_addr)
            .service_name(env!("CARGO_PKG_NAME"));
    }

    if let Err(e) = telemetry_conf
        .try_init()
        .wrap_err("failed to setup telemetry")
    {
        eprintln!("initializing composer failed:\n{e:?}");
        return ExitCode::FAILURE;
    }

    let cfg_ser = serde_json::to_string(&cfg)
        .expect("the json serializer should never fail when serializing to a string");
    eprintln!("config:\n{cfg_ser}");

    info!(config = cfg_ser, "initializing composer",);

    Composer::from_config(&cfg)
        .expect("failed creating composer")
        .run_until_stopped()
        .await;
    ExitCode::SUCCESS
}
