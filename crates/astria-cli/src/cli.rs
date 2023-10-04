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
#[derive(Subcommand)]
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

/// Manage your rollups
#[derive(Subcommand)]
pub enum RollupCommand {
    /// Manage your rollup configs
    Config {
        #[clap(subcommand)]
        command: RollupConfigCommand,
    },
}

/// Commands for managing rollup configs.
#[derive(Subcommand)]
pub enum RollupConfigCommand {
    /// Create a new rollup config
    Create(RollupConfigCreateArgs),
    /// Edit a rollup config
    Edit(RollupConfigEditArgs),
    /// Deploy a rollup config
    Deploy(RollupConfigDeployArgs),
    /// Delete a rollup config
    Delete(RollupConfigDeleteArgs),
}

#[derive(Args, Debug)]
pub struct RollupConfigCreateArgs {
    /// Path to optional config override file
    #[clap(long)]
    config_path: Option<Utf8PathBuf>,
    /// The name of the config to create
    #[clap(long)]
    pub(crate) config_name: Option<String>,
}

#[derive(Args, Debug)]
pub struct RollupConfigEditArgs {
    /// The name of the config to edit
    #[clap(long)]
    pub(crate) config_name: Option<String>,
}

#[derive(Args, Debug)]
pub struct RollupConfigDeployArgs {
    /// The name of the config to deploy
    #[clap(long)]
    pub(crate) config_name: Option<String>,
}

#[derive(Args, Debug)]
pub struct RollupConfigDeleteArgs {
    /// The name of the config to delete
    #[clap(long)]
    pub(crate) config_name: Option<String>,
}

/// Interact with the Sequencer
#[derive(Subcommand)]
pub enum SequencerCommand {
    /// Create a new Sequencer account
    Account {
        #[clap(subcommand)]
        command: SequencerAccountCommand,
    },
    /// Get the balance of a Sequencer account
    Balance {
        #[clap(subcommand)]
        command: SequencerBalanceCommand,
    },
}

#[derive(Subcommand)]
pub enum SequencerAccountCommand {
    /// Create a new sequencer account
    Create,
}

#[derive(Subcommand)]
pub enum SequencerBalanceCommand {
    /// Get the balance of a sequencer account
    Get(SequencerBalanceGetArgs),
}

#[derive(Args, Debug)]
pub struct SequencerBalanceGetArgs {
    /// The address of the sequencer account
    #[clap(long)]
    pub(crate) address: Option<String>,
}
