use clap::Subcommand;

mod account;
mod address;
mod balance;
mod block_height;
mod bridge_lock;
mod ics20_withdrawal;
mod init_bridge_account;
mod sign;
mod submit;
mod sudo;
mod threshold;
mod transfer;

use crate::command::{
    run,
    run_sync,
};

#[derive(Clone, Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) fn run(self) -> crate::command::RunCommandFut {
        match self.command {
            SubCommand::Account(account) => run(|| account.run()),
            SubCommand::Address(address) => run_sync(|| address.run()),
            SubCommand::Balance(balance) => run(|| balance.run()),
            SubCommand::BlockHeight(block_height) => run(|| block_height.run()),
            SubCommand::BridgeLock(bridge_lock) => run(|| bridge_lock.run()),
            SubCommand::InitBridgeAccount(init_bridge_account) => run(|| init_bridge_account.run()),
            SubCommand::Sudo(sudo) => run(|| sudo.run()),
            SubCommand::Transfer(transfer) => run(|| transfer.run()),
            SubCommand::Threshold(threshold) => run(|| threshold.run()),
            SubCommand::Ics20Withdrawal(ics20_withdrawal) => run(|| ics20_withdrawal.run()),

            SubCommand::Submit(submit) => run(|| submit.run()),
            SubCommand::Sign(sign) => run_sync(|| sign.run()),
        }
    }
}

/// Interact with a Sequencer node
#[derive(Clone, Debug, Subcommand)]
enum SubCommand {
    /// Commands for interacting with Sequencer accounts
    Account(account::Command),
    /// Utilities for constructing and inspecting sequencer addresses
    Address(address::Command),
    /// Commands for interacting with Sequencer balances
    Balance(balance::Command),
    /// Commands for interacting with Sequencer block heights
    #[command(name = "blockheight")]
    BlockHeight(block_height::Command),
    /// Command for transferring to a bridge account
    BridgeLock(bridge_lock::Command),
    /// Command for initializing a bridge account
    InitBridgeAccount(init_bridge_account::Command),
    /// Commands requiring authority for Sequencer
    Sudo(sudo::Command),
    /// Command for sending balance between accounts
    Transfer(transfer::Command),
    /// Commands for threshold signing
    Threshold(threshold::Command),
    /// Command for withdrawing an ICS20 asset
    Ics20Withdrawal(ics20_withdrawal::Command),
    /// Submit the signed pbjson formatted Transaction.
    Submit(submit::Command),
    /// Sign a pbjson formatted TransactionBody to produce a Transaction.
    #[expect(
        clippy::doc_markdown,
        reason = "doc comments are turned into CLI help strings which currently don't use \
                  backticks"
    )]
    Sign(sign::Command),
}
