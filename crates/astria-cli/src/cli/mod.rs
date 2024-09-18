pub(crate) mod bridge;
pub(crate) mod sequencer;

use clap::{
    Parser,
    Subcommand,
};
use color_eyre::eyre;

use crate::cli::sequencer::Command as SequencerCommand;

const DEFAULT_SEQUENCER_RPC: &str = "https://rpc.sequencer.dusk-10.devnet.astria.org";
const DEFAULT_SEQUENCER_CHAIN_ID: &str = "astria-dusk-10";

/// A CLI for deploying and managing Astria services and related infrastructure.
#[derive(Debug, Parser)]
#[command(name = "astria-cli", version)]
pub struct Cli {
    /// Sets the log level (e.g. error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info")]
    pub(crate) log_level: String,

    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

impl Cli {
    /// Parse the command line arguments
    ///
    /// # Errors
    ///
    /// * If the arguments cannot be parsed
    pub fn get_args() -> eyre::Result<Self> {
        let args = Self::parse();
        Ok(args)
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
