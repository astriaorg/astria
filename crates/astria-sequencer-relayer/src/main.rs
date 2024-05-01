use std::process::ExitCode;

use astria_eyre::eyre::WrapErr as _;
use astria_sequencer_relayer::{
    metrics_init,
    Config,
    SequencerRelayer,
    BUILD_INFO,
};
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

    eprintln!("{}", telemetry::display::json(&BUILD_INFO),);

    let cfg: Config = config::get().expect("failed to read configuration");
    eprintln!("{}", telemetry::display::json(&cfg),);

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
        eprintln!("initializing sequencer-relayer failed:\n{e:?}");
        return ExitCode::FAILURE;
    }

    info!(
        config = %telemetry::display::json(&cfg),
        "initializing sequencer relayer"
    );

    let mut sigterm = signal(SignalKind::terminate())
        .expect("setting a SIGTERM listener should always work on Unix");
    let (sequencer_relayer, shutdown_handle) =
        SequencerRelayer::new(cfg).expect("could not initialize sequencer relayer");
    let sequencer_relayer_handle = tokio::spawn(sequencer_relayer.run());

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

    if let Err(error) = sequencer_relayer_handle.await {
        error!(%error, "failed to join main sequencer relayer task");
    }

    ExitCode::SUCCESS
}
