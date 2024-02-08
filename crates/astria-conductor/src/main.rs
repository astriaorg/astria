use std::process::ExitCode;

use astria_conductor::{
    install_error_handler,
    Conductor,
    Config,
};
use eyre::WrapErr as _;
use tracing::{
    error,
    info,
};

// Following the BSD convention for failing to read config
// See here: https://freedesktop.org/software/systemd/man/systemd.exec.html#Process%20Exit%20Codes
const EX_CONFIG: u8 = 78;

#[tokio::main]
async fn main() -> ExitCode {
    install_error_handler().expect("must be able to install error formatter");

    let cfg: Config = match config::get().wrap_err("failed reading config") {
        Err(e) => {
            eprintln!("failed to start conductor:\n{e}");
            // FIXME (https://github.com/astriaorg/astria/issues/368): might have to bubble up exit codes, since we might need
            //        to exit with other exit codes if something else fails
            return ExitCode::from(EX_CONFIG);
        }
        Ok(cfg) => cfg,
    };

    if let Err(e) = telemetry::configure()
        .set_no_otel(cfg.no_otel)
        .set_force_stdout(cfg.force_stdout)
        .filter_directives(&cfg.log)
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

    let conductor = match Conductor::new(cfg).await {
        Err(e) => {
            let error: &(dyn std::error::Error + 'static) = e.as_ref();
            error!(error, "failed initializing conductor");
            return ExitCode::FAILURE;
        }
        Ok(conductor) => conductor,
    };

    conductor.run_until_stopped().await;
    info!("conductor stopped");
    ExitCode::SUCCESS
}
