use std::process::ExitCode;

use astria_bridge::{
    metrics_init,
    Conductor,
    Config,
    BUILD_INFO,
};
use astria_eyre::eyre::WrapErr as _;
use tracing::{
    error,
    info,
};

// Following the BSD convention for failing to read config
// See here: https://freedesktop.org/software/systemd/man/systemd.exec.html#Process%20Exit%20Codes
const EX_CONFIG: u8 = 78;

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
        eprintln!("initializing conductor failed:\n{e:?}");
        return ExitCode::FAILURE;
    }

    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing conductor"
    );

    let mut sigterm = signal(SignalKind::terminate())
        .expect("setting a SIGTERM listener should always work on Unix");
    let (bridge, shutdown_handle) = Bridge::new(cfg).expect("could not initialize bridge");
    let bridge_handle = tokio::spawn(bridge.run());

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

    if let Err(error) = bridge_handle.await {
        error!(%error, "failed to join main bridge task");
    }

    info!("bridge stopped");
    ExitCode::SUCCESS
}
