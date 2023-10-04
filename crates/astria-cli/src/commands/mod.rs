mod rollup;
mod sequencer;

use color_eyre::{
    eyre,
    eyre::eyre,
};

use crate::cli::{
    Cli,
    Command,
    RollupCommand,
    RollupConfigCommand,
    SequencerAccountCommand,
    SequencerBalanceCommand,
    SequencerCommand,
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
pub fn run(cli: Cli) -> eyre::Result<()> {
    if let Some(command) = cli.command {
        match command {
            Command::Rollup {
                command,
            } => match command {
                RollupCommand::Config {
                    command,
                } => match command {
                    RollupConfigCommand::Create(args) => rollup::create_config(&args)?,
                    RollupConfigCommand::Edit(args) => rollup::edit_config(&args),
                    RollupConfigCommand::Deploy(args) => rollup::deploy_config(&args),
                    RollupConfigCommand::Delete(args) => rollup::delete_config(&args),
                },
            },
            Command::Sequencer {
                command,
            } => match command {
                SequencerCommand::Account {
                    command,
                } => match command {
                    SequencerAccountCommand::Create => sequencer::create_account(),
                },
                SequencerCommand::Balance {
                    command,
                } => match command {
                    // SequencerBalanceCommand::Get(args) => sequencer::get_balance(&args)?,
                    SequencerBalanceCommand::Get(_args) => todo!(),
                },
            },
        }
    } else {
        return Err(eyre!("Error: No command specified"));
    }
    Ok(())
}
