mod rollup;
mod sequencer;

use color_eyre::{
    eyre,
    eyre::eyre,
};
use tracing::instrument;

use crate::{
    cli::{
        rollup::{
            Command as RollupCommand,
            ConfigCommand,
        },
        sequencer::{
            AccountCommand,
            BalanceCommand,
            BlockHeightCommand,
            Command as SequencerCommand,
        },
        Cli,
        Command,
    },
    commands::sequencer::{
        get_balance,
        get_block_height,
    },
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
                    ConfigCommand::Create(args) => rollup::create_config(&args)?,
                    ConfigCommand::Edit(args) => rollup::edit_config(&args),
                    ConfigCommand::Deploy(args) => rollup::deploy_config(&args)?,
                    ConfigCommand::Delete(args) => rollup::delete_config(&args)?,
                },
            },
            Command::Sequencer {
                command,
            } => match command {
                SequencerCommand::Account {
                    command,
                } => match command {
                    AccountCommand::Create => sequencer::create_account(),
                },
                SequencerCommand::Balance {
                    command,
                } => match command {
                    BalanceCommand::Get(args) => get_balance(&args).await?,
                },
                SequencerCommand::BlockHeight {
                    command,
                } => match command {
                    BlockHeightCommand::Get(args) => get_block_height(&args).await?,
                },
            },
        }
    } else {
        return Err(eyre!("Error: No command specified"));
    }
    Ok(())
}
