pub(crate) mod sequencer;

use clap::{
    Parser,
    Subcommand,
};
use color_eyre::eyre;

use crate::cli::sequencer::Command as SequencerCommand;

const DEFAULT_SEQUENCER_RPC: &str = "https://rpc.sequencer.dusk-7.devnet.astria.org";
const DEFAULT_SEQUENCER_CHAIN_ID: &str = "astria-dusk-7";

/// A CLI for deploying and managing Astria services and related infrastructure.
#[derive(Debug, Parser)]
#[command(name = "astria-cli", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
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
pub enum Command {
    Sequencer {
        #[command(subcommand)]
        command: SequencerCommand,
    },
}
