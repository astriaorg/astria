pub(crate) mod bridge;
mod sequencer;

use color_eyre::{
    eyre,
    eyre::eyre,
};

use crate::cli::{
    sequencer::{
        AccountCommand,
        AddressCommand,
        BalanceCommand,
        BlockHeightCommand,
        Command as SequencerCommand,
        FeeAssetChangeCommand,
        IbcRelayerChangeCommand,
        SudoCommand,
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
pub async fn run(cli: Cli) -> eyre::Result<()> {
    if let Some(command) = cli.command {
        match command {
            Command::Bridge {
                command,
            } => command.run().await?,
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
                SequencerCommand::Address {
                    command,
                } => match command {
                    AddressCommand::Bech32m(args) => sequencer::make_bech32m(&args)?,
                },
                SequencerCommand::Balance {
                    command,
                } => match command {
                    BalanceCommand::Get(args) => sequencer::get_balance(&args).await?,
                },
                SequencerCommand::Sudo {
                    command,
                } => match command {
                    SudoCommand::IbcRelayer {
                        command,
                    } => match command {
                        IbcRelayerChangeCommand::Add(args) => {
                            sequencer::ibc_relayer_add(&args).await?;
                        }
                        IbcRelayerChangeCommand::Remove(args) => {
                            sequencer::ibc_relayer_remove(&args).await?;
                        }
                    },
                    SudoCommand::FeeAsset {
                        command,
                    } => match command {
                        FeeAssetChangeCommand::Add(args) => sequencer::fee_asset_add(&args).await?,
                        FeeAssetChangeCommand::Remove(args) => {
                            sequencer::fee_asset_remove(&args).await?;
                        }
                    },
                    SudoCommand::ValidatorUpdate(args) => {
                        sequencer::validator_update(&args).await?;
                    }
                    SudoCommand::SudoAddressChange(args) => {
                        sequencer::sudo_address_change(&args).await?;
                    }
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
