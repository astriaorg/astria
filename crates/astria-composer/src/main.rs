use std::process::ExitCode;

use astria_composer::{
    metrics_init,
    Composer,
    Config,
    BUILD_INFO,
};
use astria_eyre::eyre::WrapErr as _;
use tracing::{
    error,
    info,
};

#[tokio::main]
async fn main() -> ExitCode {
    astria_eyre::install().expect("astria eyre hook must be the first hook installed");

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
        .set_pretty_print(cfg.pretty_print)
        .filter_directives(&cfg.log);

    if !cfg.no_metrics {
        telemetry_conf = telemetry_conf
            .metrics_addr(&cfg.metrics_http_listener_addr)
            .service_name(env!("CARGO_PKG_NAME"));
    }

    metrics_init::register();

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

    let composer = match Composer::from_config(&cfg).await {
        Err(error) => {
            error!(%error, "failed initializing Composer");
            return ExitCode::FAILURE;
        }
        Ok(composer) => composer,
    };

    return match composer.run_until_stopped().await {
        Ok(()) => {
            info!("composer stopped");
            ExitCode::SUCCESS
        }
        Err(error) => {
            error!(%error, "Composer exited with errro");
            ExitCode::FAILURE
        }
    };
}
