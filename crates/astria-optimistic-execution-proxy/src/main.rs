use std::process::ExitCode;

use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use astria_optimistic_execution_proxy::{
    Config,
    OptimisticExecutionProxy,
    BUILD_INFO,
};
use tokio::{
    select,
    signal::unix::{
        signal,
        SignalKind,
    },
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

    eprintln!("{}", telemetry::display::json(&BUILD_INFO));

    let cfg: Config = match config::get() {
        Err(err) => {
            eprintln!("failed to read configuration:\n{err:?}");
            return ExitCode::FAILURE;
        }
        Ok(cfg) => cfg,
    };
    eprintln!(
        "starting with configuration:\n{}",
        telemetry::display::json(&cfg),
    );

    let mut telemetry_conf = telemetry::configure()
        .set_no_otel(cfg.no_otel)
        .set_force_stdout(cfg.force_stdout)
        .set_pretty_print(cfg.pretty_print)
        .set_filter_directives(&cfg.log);

    if !cfg.no_metrics {
        telemetry_conf =
            telemetry_conf.set_metrics(&cfg.metrics_http_listener_addr, env!("CARGO_PKG_NAME"));
    }

    let (metrics, _telemetry_guard) = match telemetry_conf
        .try_init(&())
        .wrap_err("failed to setup telemetry")
    {
        Err(e) => {
            eprintln!("initializing optimistic execution proxy failed:\n{e:?}");
            return ExitCode::FAILURE;
        }
        Ok(metrics_and_guard) => metrics_and_guard,
    };

    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing optimistic execution proxy"
    );

    let mut optimistic_execution_proxy = match OptimisticExecutionProxy::spawn(cfg, metrics) {
        Ok(optimistic_execution_proxy) => optimistic_execution_proxy,
        Err(error) => {
            error!(%error, "failed initializing optimistic execution proxy");
            return ExitCode::FAILURE;
        }
    };

    let mut sigterm = signal(SignalKind::terminate())
        .expect("setting a SIGTERM listener should always work on Unix");

    let exit_reason = select! {
        _ = sigterm.recv() => Ok("received shutdown signal"),
        res = &mut optimistic_execution_proxy => {
            res.and_then(|()| Err(eyre!("optimistic_execution_proxy task exited unexpectedly")))
        }
    };

    shutdown(exit_reason, optimistic_execution_proxy).await
}

#[instrument(skip_all)]
async fn shutdown(
    reason: eyre::Result<&'static str>,
    mut service: OptimisticExecutionProxy,
) -> ExitCode {
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
    info!("shutdown target reached");
    exit_code
}
