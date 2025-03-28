use std::process::ExitCode;

use astria_bridge_signer::{
    BridgeSigner,
    Config,
    BUILD_INFO,
};
use astria_eyre::eyre::WrapErr as _;
use tracing::info;

#[tokio::main]
async fn main() -> ExitCode {
    astria_eyre::install().expect("astria eyre hook must be the first hook installed");

    eprintln!("{}", telemetry::display::json(&BUILD_INFO));

    let cfg: Config = match config::get() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("failed to read configuration: {e}");
            return ExitCode::FAILURE;
        }
    };
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
            eprintln!("initializing bridge signer failed:\n{e:?}");
            return ExitCode::FAILURE;
        }
        Ok(metrics_and_guard) => metrics_and_guard,
    };

    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing bridge signer"
    );

    let bridge_signer = match BridgeSigner::from_config(cfg, metrics) {
        Ok(bridge_signer) => bridge_signer,
        Err(e) => {
            eprintln!("initializing bridge signer failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = bridge_signer.run_until_stopped().await {
        eprintln!("bridge signer failed: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
