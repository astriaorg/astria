use std::process::ExitCode;

use astria_eyre::eyre::WrapErr as _;
use astria_sequencer_relayer::{
    metrics_init,
    Config,
    SequencerRelayer,
    ShutdownController,
    BUILD_INFO,
};
use tracing::info;

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

    let (_shutdown_controller, shutdown_receiver) = ShutdownController::new();
    SequencerRelayer::new(cfg, shutdown_receiver)
        .expect("could not initialize sequencer relayer")
        .run()
        .await;

    ExitCode::SUCCESS
}
