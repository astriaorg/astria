use std::process::ExitCode;

use astria_sequencer_relayer::{
    telemetry,
    Config,
    SequencerRelayer,
};
use eyre::WrapErr as _;
use tracing::info;

#[tokio::main]
async fn main() -> ExitCode {
    let cfg: Config = config::get().expect("failed to read configuration");

    if let Err(e) = telemetry::configure()
        .set_no_otel(cfg.no_otel)
        .set_force_stdout(cfg.force_stdout)
        .filter_directives(&cfg.log)
        .try_init()
        .wrap_err("failed to setup telemetry")
    {
        eprintln!("initializing sequencer-relayer failed:\n{e:?}");
        return ExitCode::FAILURE;
    }

    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing sequencer relayer"
    );

    SequencerRelayer::new(&cfg)
        .await
        .expect("could not initialize sequencer relayer")
        .run()
        .await;

    ExitCode::SUCCESS
}
