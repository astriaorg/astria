mod create;
mod delete;
mod deploy;

use color_eyre::{
    eyre,
    eyre::eyre,
};

use crate::{
    cli::{
        Cli,
        Command,
        CreateCommand,
        DeleteCelestiaCommand,
        DeleteCommand,
        DeleteRollupCommand,
        DeleteSequencerCommand,
        DeployCelestiaCommand,
        DeployCommand,
        DeployRollupCommand,
        DeploySequencerCommand,
    },
    commands::{
        create::create_sequencer_account,
        delete::{
            delete_celestia_local,
            delete_rollup_local,
            delete_rollup_remote,
            delete_sequencer_local,
        },
        deploy::{
            deploy_celestia_local,
            deploy_rollup_local,
            deploy_rollup_remote,
            deploy_sequencer_local,
        },
    },
};

/// Checks what function needs to be run and calls it with the appropriate arguments
pub fn run(cli: Cli) -> eyre::Result<()> {
    if let Some(command) = cli.command {
        match command {
            Command::Deploy {
                command,
            } => match command {
                DeployCommand::Celestia {
                    command,
                } => match command {
                    DeployCelestiaCommand::Local(args) => deploy_celestia_local(args)?,
                },
                DeployCommand::Sequencer {
                    command,
                } => match command {
                    DeploySequencerCommand::Local(args) => deploy_sequencer_local(args)?,
                },
                DeployCommand::Rollup {
                    command,
                } => match command {
                    DeployRollupCommand::Local(args) => deploy_rollup_local(args)?,
                    DeployRollupCommand::Remote(args) => deploy_rollup_remote(args)?,
                },
            },
            Command::Delete {
                command,
            } => match command {
                DeleteCommand::Celestia {
                    command,
                } => match command {
                    DeleteCelestiaCommand::Local => delete_celestia_local()?,
                },
                DeleteCommand::Sequencer {
                    command,
                } => match command {
                    DeleteSequencerCommand::Local => delete_sequencer_local()?,
                },
                DeleteCommand::Rollup {
                    command,
                } => match command {
                    DeleteRollupCommand::Local(args) => delete_rollup_local(args)?,
                    DeleteRollupCommand::Remote(args) => delete_rollup_remote(args)?,
                },
            },
            Command::Create {
                command,
            } => match command {
                CreateCommand::SequencerAccount => create_sequencer_account()?,
            },
        }
    } else {
        return Err(eyre!("Error: No command specified"));
    }
    Ok(())
}
