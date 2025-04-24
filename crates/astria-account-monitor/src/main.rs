use std::process::ExitCode;

use astria_account_monitor::{
    config::Config,
    AccountMonitor,
    BUILD_INFO,
};
use astria_eyre::eyre::WrapErr as _;

#[tokio::main]
async fn main() -> ExitCode {
    astria_eyre::install().expect("astria eyre hook must be the first hook installed");

    eprintln!(
        "{}",
        serde_json::to_string(&BUILD_INFO)
            .expect("build info is serializable because it contains only unicode fields")
    );

    let cfg: Config = config::get().expect("failed to read configuration");

    let (metrics, _telemetry_guard) = match telemetry::configure()
        .set_no_otel(cfg.no_otel)
        .set_force_stdout(cfg.force_stdout)
        .set_filter_directives(&cfg.log)
        .set_metrics(&cfg.metrics_http_listener_addr, env!("CARGO_PKG_NAME"))
        .try_init(&cfg)
        .wrap_err("failed to setup telemetry")
    {
        Err(error) => {
            eprintln!("failed to setup telemetry: {error}");
            return ExitCode::FAILURE;
        }
        Ok(telemetry_conf) => telemetry_conf,
    };

    let account_monitor = match AccountMonitor::new(cfg, metrics) {
        Err(_) => return ExitCode::FAILURE,
        Ok(account_monitor) => account_monitor,
    };

    match account_monitor.run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("account monitor exited unexpectedly: {error}");
            ExitCode::FAILURE
        }
    }
}
