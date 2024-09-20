use std::process::ExitCode;

use astria_cli::{
    cli::Cli,
    commands,
};
use color_eyre::eyre;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .pretty()
        .with_writer(std::io::stderr)
        .init();

    if let Err(err) = run().await {
        eprintln!("{err:?}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

async fn run() -> eyre::Result<()> {
    // let config: Config = config::get_networks_config()?;

    // Parse the TOML string into our Config struct
    let args = Cli::get_args()?;

    // // Validate the selected network name
    // if config.validate_network(args.network.clone()) {
    //     println!("network: {:?}", args.network);
    //     if let Some(network_config) = config.get_network(args.network.clone()) {
    //         args.set_network_config(&network_config.clone());
    //     } else {
    //         println!("Network config not found");
    //     }
    //     // args.set_network_config(config.get_network(args.network.clone()));
    // } else {
    //     println!(
    //         "Network is not valid. Expected one of: {:?}",
    //         config.get_valid_networks()
    //     );
    // }

    commands::run(args).await
}
