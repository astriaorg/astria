use std::process::ExitCode;

use astria_bridge_withdrawer::{
    BridgeWithdrawer,
    Config,
    BUILD_INFO,
};
use astria_eyre::eyre::WrapErr as _;
use tokio::signal::unix::{
    signal,
    SignalKind,
};
use tracing::{
    error,
    info,
    warn,
};

#[tokio::main]
async fn main() -> ExitCode {
    astria_eyre::install().expect("astria eyre hook must be the first hook installed");

    eprintln!("{}", telemetry::display::json(&BUILD_INFO));

    let cfg: Config = config::get().expect("failed to read configuration");
    eprintln!("{}", telemetry::display::json(&cfg),);

    let mut telemetry_conf = telemetry::configure()
        .set_no_otel(cfg.no_otel)
        .set_force_stdout(cfg.force_stdout)
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
            eprintln!("initializing bridge withdrawer failed:\n{e:?}");
            return ExitCode::FAILURE;
        }
        Ok(metrics_and_guard) => metrics_and_guard,
    };

    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing bridge withdrawer"
    );

    let mut sigterm = signal(SignalKind::terminate())
        .expect("setting a SIGTERM listener should always work on Unix");
    let (withdrawer, shutdown_handle) = match BridgeWithdrawer::new(cfg, metrics).await {
        Err(error) => {
            error!(%error, "failed initializing bridge withdrawer");
            return ExitCode::FAILURE;
        }
        Ok(handles) => handles,
    };
    let withdrawer_handle = tokio::spawn(withdrawer.run());

    let shutdown_token = shutdown_handle.token();
    tokio::select!(
        _ = sigterm.recv() => {
            // We don't care about the result (i.e. whether there could be more SIGTERM signals
            // incoming); we just want to shut down as soon as we receive the first `SIGTERM`.
            info!("received SIGTERM, issuing shutdown to all services");
            shutdown_handle.shutdown();
        }
        () = shutdown_token.cancelled() => {
            warn!("stopped waiting for SIGTERM");
        }
    );

    if let Err(error) = withdrawer_handle.await {
        error!(%error, "failed to join main withdrawer task");
    }

    info!("withdrawer stopped");
    ExitCode::SUCCESS
}
