use astria_sequencer::{
    config,
    Sequencer,
};
use tracing::info;
use std::process::ExitCode;

// Following the BSD convention for failing to read config
// See here: https://freedesktop.org/software/systemd/man/systemd.exec.html#Process%20Exit%20Codes
const EX_CONFIG: u8 = 78;

#[tokio::main]
async fn main() -> ExitCode {
    let config = match config::get() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("failed to read configuration: {e:?}");
            return ExitCode::from(EX_CONFIG);
        }
    };
    telemetry::init(std::io::stdout, &config.log).expect("failed to initialize telemetry");
    info!(
        config = serde_json::to_string(&config).expect("serializing to a string cannot fail"),
        "initializing sequencer"
    );
    Sequencer::run_until_stopped(config)
        .await
        .expect("failed to run sequencer");

    info!("Sequencer stopped");
    ExitCode::SUCCESS
}
