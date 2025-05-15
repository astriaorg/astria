use clap::{
    Parser,
    Subcommand,
};

use super::{
    activation_point_estimator,
    blob_parser,
    genesis_example,
    genesis_parser,
};

/// Utilities for working with the Astria sequencer network
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[expect(
    clippy::large_enum_variant,
    reason = "this doesn't matter for this enum"
)]
pub enum Command {
    /// Copy genesis state to a JSON file
    #[command(arg_required_else_help = true)]
    CopyGenesisState(genesis_parser::Args),

    /// Generate an example sequencer genesis state
    GenerateGenesisState(genesis_example::Args),

    /// Parse blob data from an arg, a file, or stdin
    #[command(arg_required_else_help = true)]
    ParseBlob(blob_parser::Args),

    /// Estimate the activation point of an upgrade to schedule or check an upcoming one
    #[command(arg_required_else_help = true)]
    EstimateActivationPoint(activation_point_estimator::Args),
}

#[must_use]
pub fn get() -> Command {
    Cli::parse().command
}
