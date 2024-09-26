use clap::Subcommand;
use color_eyre::eyre;

mod account;
mod address;
mod balance;
mod block_height;
mod bridge_lock;
mod init_bridge_account;
mod sudo;
mod transfer;

#[derive(Debug, clap::Args)]
pub(super) struct Args {
    #[command(subcommand)]
    command: Command,
}

impl Args {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            Command::Account(account) => account.run().await,
            Command::Address(address) => address.run(),
            Command::Balance(balance) => balance.run().await,
            Command::BlockHeight(block_height) => block_height.run().await,
            Command::BridgeLock(bridge_lock) => bridge_lock.run().await,
            Command::InitBridgeAccount(init_bridge_account) => init_bridge_account.run().await,
            Command::Sudo(sudo) => sudo.run().await,
            Command::Transfer(transfer) => transfer.run().await,
        }
    }
}

/// Interact with a Sequencer node
#[derive(Debug, Subcommand)]
enum Command {
    /// Commands for interacting with Sequencer accounts
    Account(account::Args),
    /// Utilities for constructing and inspecting sequencer addresses
    Address(address::Args),
    /// Commands for interacting with Sequencer balances
    Balance(balance::Args),
    /// Commands for interacting with Sequencer block heights
    #[command(name = "blockheight")]
    BlockHeight(block_height::Args),
    /// Command for transferring to a bridge account
    BridgeLock(bridge_lock::Args),
    /// Command for initializing a bridge account
    InitBridgeAccount(init_bridge_account::Args),
    /// Commands requiring authority for Sequencer
    Sudo(sudo::Args),
    /// Command for sending balance between accounts
    Transfer(transfer::Args),
}
