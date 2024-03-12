use std::process::ExitCode;

use astria_eyre::eyre::WrapErr as _;
use astria_sequencer_relayer::{
    metrics_init,
    telemetry,
    Config,
    SequencerRelayer,
    BUILD_INFO,
};
use tracing::info;

#[tokio::main]
async fn main() -> ExitCode {
    astria_eyre::install().expect("astria eyre hook must be the first hook installed");

    eprintln!(
        "{}",
        serde_json::to_string(&BUILD_INFO)
            .expect("build info is serializable because it contains only unicode fields")
    );
    let cfg: Config = config::get().expect("failed to read configuration");

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

    SequencerRelayer::new(&cfg)
        .await
        .expect("could not initialize sequencer relayer")
        .run()
        .await;

    ExitCode::SUCCESS
}
