pub(crate) mod rollup;
pub(crate) mod sequencer;

use clap::{
    Parser,
    Subcommand,
};
use color_eyre::eyre;

use crate::cli::{
    rollup::Command as RollupCommand,
    sequencer::Command as SequencerCommand,
};

const DEFAULT_SEQUENCER_RPC: &str = "https://rpc.sequencer.dusk-3.devnet.astria.org";

/// A CLI for deploying and managing Astria services and related infrastructure.
#[derive(Debug, Parser)]
#[clap(name = "astria-cli", version)]
pub struct Cli {
    #[clap(subcommand)]
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
    Rollup {
        #[clap(subcommand)]
        command: RollupCommand,
    },
    Sequencer {
        #[clap(subcommand)]
        command: SequencerCommand,
    },
}
