use std::process::ExitCode;

use astria_account_monitor::{
    config::Config,
    AccountMonitor,
    BUILD_INFO,
};
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use tokio::signal::unix::{
    signal,
    SignalKind,
};
use tracing::{
    error,
    info,
    instrument,
    warn,
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

    let exit_reasion = tokio::select! {
        _ = sigterm.recv() => {
                Ok("received shutdown signal")
        },
        res = &mut account_monitor => {
            res.and_then(|()| Err(eyre::eyre!("account monitor task exited unexpectedly")))
        }
    };

    shutdown(exit_reasion, account_monitor).await
}

#[instrument(skip_all)]
async fn shutdown(reason: eyre::Result<&'static str>, service: AccountMonitor) -> ExitCode {
    let message = "shutting down";
    let exit_code = match reason {
        Ok(reason) => {
            info!(reason, message);
            if let Err(error) = service.shutdown().await {
                warn!(%error, "encountered errors during shutdown");
            };
            ExitCode::SUCCESS
        }
        Err(reason) => {
            error!(%reason, message);
            ExitCode::FAILURE
        }
    };
    exit_code
}
