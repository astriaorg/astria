mod blockheight;
mod nonce;

use clap::{
    ArgMatches,
    Command,
};
use color_eyre::eyre;

pub(crate) fn command() -> Command {
    Command::new("query")
        .about("Query the sequencer")
        .subcommand(blockheight::command())
        .subcommand(nonce::command())
}

pub(crate) async fn run(matches: &ArgMatches) -> eyre::Result<()> {
    match matches.subcommand() {
        Some(("blockheight", args)) => blockheight::run(args).await,
        Some(("nonce", args)) => nonce::run(args).await,
        // Some(("subcommand2", sub_matches)) => subcommand2::run(sub_matches, config),
        _ => Err(eyre::eyre!("Unknown subcommand")),
    }
}
