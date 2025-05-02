use std::process::ExitCode;

use astria_account_monitor::{
    config::Config,
    AccountMonitor,
    BUILD_INFO,
};
use astria_eyre::eyre::WrapErr as _;
use tokio::signal::unix::{
    signal,
    SignalKind,
};
use tracing::warn;

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
            eprintln!("failed to setup telemetry:\n {error}");
            return ExitCode::FAILURE;
        }
        Ok(telemetry_conf) => telemetry_conf,
    };

    let mut account_monitor = match AccountMonitor::spawn(cfg, metrics) {
        Err(error) => {
            eprintln!("failed to start account monitor:\n {error}");
            return ExitCode::FAILURE;
        }
        Ok(account_monitor) => account_monitor,
    };

    let mut sigterm = signal(SignalKind::terminate())
        .expect("setting a SIGTERM listener should always work on Unix");

    tokio::select! {
        _ = sigterm.recv() => {
            match account_monitor.shutdown().await {
                Ok(()) => {
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("failed to shutdown account monitor:\n {err}");
                    ExitCode::FAILURE
                }
            }
        },
        res = &mut account_monitor => {
            match res {
                Ok(()) => {
                    eprintln!("account monitor exited unexpectedly");
                    ExitCode::FAILURE
                }
                Err(err) => {
                    eprintln!("account monitor exited unexpectedly with an error:\n {err}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
