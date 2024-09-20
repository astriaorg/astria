pub(crate) mod bridge;
pub(crate) mod config;
pub(crate) mod sequencer;

use clap::{
    Parser,
    Subcommand,
};
use color_eyre::eyre;

use super::*;
use crate::cli::{
    config::{
        Config,
        NetworkConfig,
    },
    sequencer::Command as SequencerCommand,
};

// const DEFAULT_SEQUENCER_RPC: &str = "https://rpc.sequencer.dusk-10.devnet.astria.org";
// const DEFAULT_SEQUENCER_CHAIN_ID: &str = "astria-dusk-10";

/// A CLI for deploying and managing Astria services and related infrastructure.
#[derive(Debug, Parser)]
#[command(name = "astria-cli", version)]
pub struct Cli {
    /// Sets the log level (e.g. error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info")]
    pub(crate) log_level: String,

    /// Select the network you want to use
    #[arg(short, long, default_value = "dawn")]
    pub network: String,

    #[command(subcommand)]
    pub(crate) command: Option<Command>,

    #[clap(skip)]
    pub(crate) network_config: Option<NetworkConfig>,
}

impl Cli {
    /// Parse the command line arguments
    ///
    /// # Errors
    ///
    /// * If the arguments cannot be parsed
    pub fn get_args() -> eyre::Result<Self> {
        let mut args = Self::parse();
        // println!("")

        let config: Config = config::get_networks_config()?;

        // Validate the selected network name
        if config.validate_network(args.network.clone()) {
            println!("network: {:?}", args.network);
            if let Some(network_config) = config.get_network(args.network.clone()) {
                args.set_network_config(network_config.clone());
            } else {
                println!("Network config not found");
            }
            // args.set_network_config(config.get_network(args.network.clone()));
        } else {
            println!(
                "Network is not valid. Expected one of: {:?}",
                config.get_valid_networks()
            );
        }

        Ok(args)
    }

    pub fn set_network_config(&mut self, network_config: NetworkConfig) {
        self.network_config = Some(network_config);
    }
}

/// Commands that can be run
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    Bridge {
        #[command(subcommand)]
        command: bridge::Command,
    },
    Sequencer {
        #[command(subcommand)]
        command: SequencerCommand,
    },
}
