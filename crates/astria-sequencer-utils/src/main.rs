use astria_sequencer_utils::{
    config::{
        Command,
        Config,
    },
    genesis_parser::GenesisParser,
    telemetry,
};
use tracing::info;

#[tokio::main]
async fn main() {
    let config = Config::get();
    telemetry::init(std::io::stdout).expect("failed to initialize telemetry");

    match config.cmd {
        Command::GenesisParser(args) => {
            info!(
                file_args = serde_json::to_string(&args).unwrap(),
                "running genesis parser"
            );
            GenesisParser::propigate_data(args)
                .await
                .expect("failed to propagate data");
            info!("genesis parsing complete")
        }
    }
}
