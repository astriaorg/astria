use std::{
    process::ExitCode,
    time::Duration,
};

use astria_account_monitor::{
    config::Config,
    AccountMonitor,
    BUILD_INFO,
};
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use tokio::{
    signal::unix::{
        signal,
        SignalKind,
    },
    time::timeout,
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
    eprintln!(
        "starting with configuration:\n{}",
        telemetry::display::json(&cfg),
    );

    let (metrics, _telemetry_guard) = match telemetry::configure()
        .set_no_otel(cfg.no_otel)
        .set_force_stdout(cfg.force_stdout)
        .set_filter_directives(&cfg.log)
        .set_metrics(&cfg.metrics_http_listener_addr, env!("CARGO_PKG_NAME"))
        .try_init(&cfg)
        .wrap_err("failed to setup telemetry")
    {
        Err(error) => {
            eprintln!("failed to setup telemetry:\n{error}");
            return ExitCode::FAILURE;
        }
        Ok(telemetry_conf) => telemetry_conf,
    };

    let mut account_monitor = match AccountMonitor::spawn(cfg, metrics) {
        Err(error) => {
            eprintln!("failed to start account monitor:\n{error}");
            return ExitCode::FAILURE;
        }
        Ok(account_monitor) => account_monitor,
    };

    let mut sigterm = signal(SignalKind::terminate())
        .expect("setting a SIGTERM listener should always work on Unix");

    let exit_reason = tokio::select! {
        _ = sigterm.recv() => {
                Ok("received shutdown signal")
        },
        res = &mut account_monitor => {
            // XXX: account monitor should never exit unless told, so it returning Ok(()) is not expected
            res.and_then(|()| Err(eyre::eyre!("account monitor task exited unexpectedly")))
        }
    };

    shutdown(exit_reason, account_monitor).await
}

#[instrument(skip_all)]
async fn shutdown(reason: eyre::Result<&'static str>, service: AccountMonitor) -> ExitCode {
    let message = "shutting down";
    match reason {
        Ok(reason) => {
            info!(reason, message);
            match timeout(Duration::from_secs(29), service.shutdown()).await {
                Ok(shutdown_result) => {
                    if let Err(error) = shutdown_result {
                        warn!(%error, "encountered errors during shutdown");
                    }
                }
                Err(_) => {
                    warn!("service shutdown timed out");
                }
            }
            ExitCode::SUCCESS
        }
        Err(reason) => {
            error!(%reason, message);
            ExitCode::FAILURE
        }
    }
}
