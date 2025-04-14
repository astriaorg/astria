use std::process::ExitCode;

use astria_account_monitor::{
    config::Config,
    AccountMonitor,
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

    let cfg: Config = config::get().expect("failed to read configuration");

    let mut telemetry_conf = telemetry::configure()
        .set_no_otel(cfg.no_otel)
        .set_force_stdout(cfg.force_stdout)
        .set_filter_directives(&cfg.log);

    if !cfg.no_metrics {
        telemetry_conf =
            telemetry_conf.set_metrics(&cfg.metrics_http_listener_addr, env!("CARGO_PKG_NAME"));
    }

    let (metrics, _telemetry_guard) = match telemetry_conf
        .try_init(&cfg)
        .wrap_err("failed to setup telemetry")
    {
        Err(e) => {
            eprintln!("initializing account monitor failed:\n{e:?}");
            return ExitCode::FAILURE;
        }
        Ok(metrics_and_guard) => metrics_and_guard,
    };

    let account_monitor = match AccountMonitor::new(cfg, metrics) {
        Ok(account_monitor) => account_monitor,
        Err(e) => {
            error!(%e, "failed initializing AccountMonitor");
            return ExitCode::FAILURE;
        }
    };
    return match account_monitor.run().await {
        Ok(()) => {
            info!("Account monitor stopped");
            ExitCode::SUCCESS
        }
        Err(error) => {
            error!(%error, "Account monitor exited with error");
            ExitCode::FAILURE
        }
    };
}
