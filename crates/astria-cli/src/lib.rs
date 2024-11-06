#![allow(
    clippy::large_enum_variant,
    reason = "the CLI contains enums with diverging variants. These are oneshot types that
              are not expected to be copied, cloned, or passed around. Therefore large differences \
              between enum variants are not expected to cause performance issues."
)]

mod bridge;
mod command;
mod output;
mod sequencer;
mod utils;

use clap::{
    Parser,
    Subcommand,
};
use color_eyre::eyre;

const DEFAULT_SEQUENCER_RPC: &str = "https://rpc.sequencer.dusk-10.devnet.astria.org";
const DEFAULT_SEQUENCER_CHAIN_ID: &str = "astria-dusk-10";

/// Run commands against the Astria network.
#[derive(Debug, Parser)]
#[command(name = "astria-cli", version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    /// Runs the Astria CLI.
    ///
    /// This is the only entry point into the Astria CLI.
    ///
    /// # Errors
    ///
    /// Returns various errors if executing a subcommand fails. The errors are
    /// not explicitly listed here.
    pub async fn run() -> eyre::Result<()> {
        let cli = Self::parse();
        let output = match cli.command {
            Command::Bridge(bridge) => command::run(move || bridge.run()).await?,
            Command::Sequencer(sequencer) => command::run(move || sequencer.run()).await?,
        };
        println!("{output}");
        Ok(())
    }
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// Collect events from a rollup and submit to Sequencer.
    Bridge(bridge::Command),
    /// Interact with Sequencer.
    Sequencer(sequencer::Command),
}
