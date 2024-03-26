mod rollup;
mod sequencer;

use color_eyre::{
    eyre,
    eyre::eyre,
};
use tracing::instrument;

use crate::cli::{
    rollup::{
        Command as RollupCommand,
        ConfigCommand,
        DeploymentCommand,
    },
    sequencer::{
        AccountCommand,
        BalanceCommand,
        BlockHeightCommand,
        Command as SequencerCommand,
    },
    Cli,
    Command,
};

/// Checks what function needs to be run and calls it with the appropriate arguments
///
/// # Arguments
///
/// * `cli` - The arguments passed to the command
///
/// # Errors
///
/// * If no command is specified
///
/// # Panics
///
/// * If the command is not recognized
#[instrument]
pub async fn run(cli: Cli) -> eyre::Result<()> {
    if let Some(command) = cli.command {
        match command {
            Command::Rollup {
                command,
            } => match command {
                RollupCommand::Config {
                    command,
                } => match command {
                    ConfigCommand::Create(args) => rollup::create_config(&args).await?,
                    ConfigCommand::Edit(args) => rollup::edit_config(&args)?,
                    ConfigCommand::Delete(args) => rollup::delete_config(&args)?,
                },
                RollupCommand::Deployment {
                    command,
                } => match command {
                    DeploymentCommand::Create(args) => rollup::create_deployment(&args)?,
                    DeploymentCommand::Delete(args) => rollup::delete_deployment(&args)?,
                    DeploymentCommand::List => rollup::list_deployments(),
                },
            },
            Command::Sequencer {
                command,
            } => match command {
                SequencerCommand::Account {
                    command,
                } => match command {
                    AccountCommand::Create => sequencer::create_account(),
                    AccountCommand::Balance(args) => sequencer::get_balance(&args).await?,
                    AccountCommand::Nonce(args) => sequencer::get_nonce(&args).await?,
                },
                SequencerCommand::Balance {
                    command,
                } => match command {
                    BalanceCommand::Get(args) => sequencer::get_balance(&args).await?,
                },
                SequencerCommand::Transfer(args) => sequencer::send_transfer(&args).await?,
                SequencerCommand::BlockHeight {
                    command,
                } => match command {
                    BlockHeightCommand::Get(args) => sequencer::get_block_height(&args).await?,
                },
                SequencerCommand::InitBridgeAccount(args) => {
                    sequencer::init_bridge_account(&args).await?;
                }
                SequencerCommand::BridgeLock(args) => sequencer::bridge_lock(&args).await?,
            },
        }
    } else {
        return Err(eyre!("Error: No command specified"));
    }
    Ok(())
}
