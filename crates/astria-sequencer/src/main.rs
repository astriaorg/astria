use std::process::ExitCode;

use astria_sequencer::{
    Config,
    Sequencer,
};
use tracing::info;

// Following the BSD convention for failing to read config
// See here: https://freedesktop.org/software/systemd/man/systemd.exec.html#Process%20Exit%20Codes
const EX_CONFIG: u8 = 78;

#[tokio::main]
async fn main() -> ExitCode {
    let config: Config = match config::get() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("failed to read configuration:\n{e:?}");
            return ExitCode::from(EX_CONFIG);
        }
    };
    let metrics_addr = if config.metrics_enabled {
        Some(config.prometheus_http_listener_addr)
    } else {
        None
    };
    telemetry::init(std::io::stdout, &config.log, metrics_addr)
        .expect("failed to initialize telemetry");
    info!(
        config = serde_json::to_string(&config).expect("serializing to a string cannot fail"),
        "initializing sequencer"
    );

    #[cfg(feature = "mint")]
    if config.enable_mint {
        tokio::spawn(async {
            let duration = std::time::Duration::from_secs(5);
            loop {
                eprintln!("MINT FEATURE IS ENABLED!");
                eprintln!("do not enable minting in production!");
                tracing::warn!("MINT FEATURE IS ENABLED!");
                tracing::warn!("do not enable minting in production!");
                tokio::time::sleep(duration).await;
            }
        });
    }

    Sequencer::run_until_stopped(config)
        .await
        .expect("failed to run sequencer");

    info!("Sequencer stopped");
    ExitCode::SUCCESS
}
