use std::process::ExitCode;

use astria_conductor::{
    Conductor,
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

// Following the BSD convention for failing to read config
// See here: https://freedesktop.org/software/systemd/man/systemd.exec.html#Process%20Exit%20Codes
const EX_CONFIG: u8 = 78;

#[tokio::main]
async fn main() -> ExitCode {
    astria_eyre::install().expect("astria eyre hook must be the first hook installed");

    eprintln!(
        "{}",
        serde_json::to_string(&BUILD_INFO)
            .expect("build info is serializable because it contains only unicode fields")
    );

    let cfg: Config = match config::get().wrap_err("failed reading config") {
        Err(e) => {
            eprintln!("failed to start conductor:\n{e}");
            // FIXME (https://github.com/astriaorg/astria/issues/368):
            //       might have to bubble up exit codes, since we might need
            //       to exit with other exit codes if something else fails
            return ExitCode::from(EX_CONFIG);
        }
        Ok(cfg) => cfg,
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
            .register_metrics(|| {}); // conductor currently has no metrics
    }

    let _telemetry_guard = match telemetry_conf
        .try_init()
        .wrap_err("failed to setup telemetry")
    {
        Err(e) => {
            eprintln!("initializing conductor failed:\n{e:?}");
            return ExitCode::FAILURE;
        }
        Ok(guard) => guard,
    };

    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing conductor"
    );

    let conductor = match Conductor::new(cfg) {
        Err(error) => {
            error!(%error, "failed initializing conductor");
            return ExitCode::FAILURE;
        }
        Ok(conductor) => conductor,
    };

    let mut sigterm = signal(SignalKind::terminate())
        .expect("setting a SIGTERM listener should always work on unix; is this running on unix?");
    let mut handle = conductor.spawn();

    select!(
        _ = sigterm.recv() => {
            info!("received SIGTERM; shutting down Conductor");
            if let Err(error) = handle.shutdown().await {
                warn!(%error, "encountered an error while shutting down");
            }
            info!("conductor stopped");
            ExitCode::SUCCESS
        }

        res = &mut handle => {
            error!(
                error = res.err().map(tracing::field::display),
                "conductor task exited unexpectedly",
            );
            ExitCode::FAILURE
        }
    )
}
