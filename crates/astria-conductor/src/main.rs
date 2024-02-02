use std::process::ExitCode;

use astria_conductor::{
    Conductor,
    Config,
};
use tracing::{
    error,
    info,
};

// Following the BSD convention for failing to read config
// See here: https://freedesktop.org/software/systemd/man/systemd.exec.html#Process%20Exit%20Codes
const EX_CONFIG: u8 = 78;

#[tokio::main]
async fn main() -> ExitCode {
    let cfg: Config = match config::get() {
        Err(e) => {
            eprintln!("failed reading config:\n{e:?}");
            // FIXME (https://github.com/astriaorg/astria/issues/368): might have to bubble up exit codes, since we might need
            //        to exit with other exit codes if something else fails
            return ExitCode::from(EX_CONFIG);
        }
        Ok(cfg) => cfg,
    };
    let metrics_conf = if cfg.metrics_enabled {
        Some(telemetry::MetricsConfig {
            addr: cfg.prometheus_http_listener_addr,
            labels: Some(vec![("service".into(), "astria-conductor".into())]),
            buckets: None,
        })
    } else {
        None
    };
    if let Err(err) = telemetry::init(std::io::stdout, &cfg.log, metrics_conf) {
        eprintln!(
            "failed initializing config with filter directive `{log}`\n{err:?}",
            log = cfg.log,
            err = err,
        );
        return ExitCode::FAILURE;
    };

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
