use std::process::ExitCode;

use astria_auctioneer::{
    Auctioneer,
    Config,
    BUILD_INFO,
};
use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
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

    eprintln!("{}", astria_telemetry::display::json(&BUILD_INFO));

    let cfg: Config = match config::get() {
        Err(err) => {
            eprintln!("failed to read configuration:\n{err:?}");
            return ExitCode::FAILURE;
        }
        Ok(cfg) => cfg,
    };
    eprintln!(
        "starting with configuration:\n{}",
        astria_telemetry::display::json(&cfg),
    );

    let mut astria_telemetry_conf = astria_telemetry::configure()
        .set_no_otel(cfg.no_otel)
        .set_force_stdout(cfg.force_stdout)
        .set_filter_directives(&cfg.log);

    if !cfg.no_metrics {
        astria_telemetry_conf = astria_telemetry_conf
            .set_metrics(&cfg.metrics_http_listener_addr, env!("CARGO_PKG_NAME"));
    }

    let (metrics, _astria_telemetry_guard) = match astria_telemetry_conf
        .try_init(&())
        .wrap_err("failed to setup astria_telemetry")
    {
        Err(e) => {
            eprintln!("initializing auctioneer failed:\n{e:?}");
            return ExitCode::FAILURE;
        }
        Ok(metrics_and_guard) => metrics_and_guard,
    };

    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing auctioneer"
    );

    let mut auctioneer = match Auctioneer::spawn(cfg, metrics) {
        Ok(auctioneer) => auctioneer,
        Err(error) => {
            error!(%error, "failed initializing auctioneer");
            return ExitCode::FAILURE;
        }
    };

    let mut sigterm = signal(SignalKind::terminate())
        .expect("setting a SIGTERM listener should always work on Unix");

    let exit_reason = select! {
        _ = sigterm.recv() => Ok("received shutdown signal"),
        res = &mut auctioneer => {
            res.and_then(|()| Err(eyre!("auctioneer task exited unexpectedly")))
        }
    };

    shutdown(exit_reason, auctioneer).await
}

#[instrument(skip_all)]
async fn shutdown(reason: eyre::Result<&'static str>, mut service: Auctioneer) -> ExitCode {
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
