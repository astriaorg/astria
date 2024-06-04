use std::process::ExitCode;

use anyhow::Context as _;
use astria_sequencer::{
    metrics_init,
    Config,
    Sequencer,
    BUILD_INFO,
};
use tracing::info;

// Following the BSD convention for failing to read config
// See here: https://freedesktop.org/software/systemd/man/systemd.exec.html#Process%20Exit%20Codes
const EX_CONFIG: u8 = 78;

#[tokio::main]
async fn main() -> ExitCode {
    eprintln!(
        "{}",
        serde_json::to_string(&BUILD_INFO)
            .expect("build info is serializable because it contains only unicode fields")
    );

    let cfg: Config = match config::get() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("failed to read configuration:\n{e:?}");
            return ExitCode::from(EX_CONFIG);
        }
    };
    let mut telemetry_conf = telemetry::configure()
        .set_no_otel(cfg.no_otel)
        .set_force_stdout(cfg.force_stdout)
        .set_pretty_print(cfg.pretty_print)
        .filter_directives(&cfg.log);
    if !cfg.no_metrics {
        telemetry_conf = telemetry_conf
            .metrics_addr(&cfg.metrics_http_listener_addr)
            .service_name(env!("CARGO_PKG_NAME"))
            .register_metrics(metrics_init::register);
    }

    let _telemetry_guard = match telemetry_conf
        .try_init()
        .context("failed to setup telemetry")
    {
        Err(e) => {
            eprintln!("initializing sequencer failed:\n{e:?}");
            return ExitCode::FAILURE;
        }
        Ok(guard) => guard,
    };

    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing sequencer"
    );

    Sequencer::run_until_stopped(cfg)
        .await
        .expect("failed to run sequencer");

    info!("Sequencer stopped");
    ExitCode::SUCCESS
}
