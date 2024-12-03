use std::process::ExitCode;

use astria_auctioneer::{
    Auctioneer,
    Config,
    BUILD_INFO,
};
use astria_eyre::eyre::WrapErr as _;
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

    select! {
        _ = sigterm.recv() => {
            info!("received SIGTERM; shutting down");
            if let Err(error) = auctioneer.shutdown().await {
                warn!(%error, "encountered an error while shutting down");
            }
            info!("auctioneer stopped");
            ExitCode::SUCCESS
        }

        res = &mut auctioneer => {
            error!(
                error = res.err().map(tracing::field::display),
                "auctioneer task exited unexpectedly"
            );
            ExitCode::FAILURE
        }
    }
}
