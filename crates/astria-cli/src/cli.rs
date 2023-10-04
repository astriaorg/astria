use camino::Utf8PathBuf;
use clap::{
    Args,
    Parser,
    Subcommand,
};
use color_eyre::eyre;

/// A CLI for deploying and managing Astria services and related infrastructure.
#[derive(Parser)]
#[clap(name = "astria-cli", version)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    pub fn get_args() -> eyre::Result<Self> {
        let args = Self::parse();
        Ok(args)
    }
}

/// Commands that can be run
#[derive(Subcommand)]
pub enum Command {
    Deploy {
        #[clap(subcommand)]
        command: DeployCommand,
    },
    Delete {
        #[clap(subcommand)]
        command: DeleteCommand,
    },
    Create {
        #[clap(subcommand)]
        command: CreateCommand,
    },
}

/// Deploy Celestia, Sequencer, or Rollup
#[derive(Subcommand)]
pub enum DeployCommand {
    /// Deploy a Celestia node
    Celestia {
        #[clap(subcommand)]
        command: DeployCelestiaCommand,
    },
    /// Deploy a Sequencer node
    Sequencer {
        #[clap(subcommand)]
        command: DeploySequencerCommand,
    },
    /// Deploy a rollup on the Astria Shared Sequencer Network
    Rollup {
        #[clap(subcommand)]
        command: DeployRollupCommand,
    },
}

#[derive(Subcommand)]
pub enum DeployCelestiaCommand {
    /// Deploy a local Celestia chart
    Local(DeployCelestiaArgs),
}

#[derive(Args, Debug)]
pub struct DeployCelestiaArgs {
    /// Path to optional config override file
    #[clap(long)]
    config: Option<Utf8PathBuf>,
    // TODO - add all options
}

#[derive(Subcommand)]
pub enum DeploySequencerCommand {
    /// Deploy a local Sequencer chart
    Local(DeploySequencerArgs),
}

#[derive(Args, Debug)]
pub struct DeploySequencerArgs {
    /// Path to optional config override file
    #[clap(long)]
    config: Option<Utf8PathBuf>,
    // TODO - add all options
}

/// Deploy a rollup
#[derive(Subcommand)]
pub enum DeployRollupCommand {
    /// Deploy a rollup on your local machine
    Local(DeployRollupArgs),

    /// Deploy a rollup on a remote machine
    Remote(DeployRollupArgs),
}

#[derive(Args, Debug)]
pub struct DeployRollupArgs {
    /// Path to optional config override file
    #[clap(long)]
    config: Option<Utf8PathBuf>,
    /// The name of the rollup to deploy
    #[clap(long)]
    pub(crate) name: Option<String>,
    /// The chain ID of the rollup
    #[clap(long)]
    pub(crate) chain_id: Option<String>,
}

/// Deletes Celestia, Sequencer, or Rollup
#[derive(Subcommand)]
pub enum DeleteCommand {
    /// Delete a Celestia node
    Celestia {
        #[clap(subcommand)]
        command: DeleteCelestiaCommand,
    },
    /// Delete a Sequencer node
    Sequencer {
        #[clap(subcommand)]
        command: DeleteSequencerCommand,
    },
    /// Delete a rollup
    Rollup {
        #[clap(subcommand)]
        command: DeleteRollupCommand,
    },
}

#[derive(Subcommand)]
pub enum DeleteCelestiaCommand {
    /// Delete a local Celestia chart
    Local,
}

#[derive(Subcommand)]
pub enum DeleteSequencerCommand {
    /// Delete a local Sequencer chart
    Local,
}

#[derive(Subcommand)]
pub enum DeleteRollupCommand {
    /// Delete a rollup on your local machine
    Local(DeleteRollupArgs),
    /// Delete a rollup on a remote machine
    Remote(DeleteRollupArgs),
}

#[derive(Args, Debug)]
pub struct DeleteRollupArgs {
    /// The name of the rollup to delete
    #[clap(long)]
    pub(crate) name: Option<String>,
}

#[derive(Subcommand)]
pub enum CreateCommand {
    /// Create a new Sequencer account
    SequencerAccount,
}
