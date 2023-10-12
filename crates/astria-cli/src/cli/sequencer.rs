use std::str::FromStr;

use astria_sequencer_client::Address;
use clap::{
    Args,
    Subcommand,
};
use color_eyre::{
    eyre,
    eyre::Context,
};

/// Interact with a Sequencer node
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Commands for interacting with Sequencer accounts
    Account {
        #[clap(subcommand)]
        command: AccountCommand,
    },
    /// Commands for interacting with Sequencer balances
    Balance {
        #[clap(subcommand)]
        command: BalanceCommand,
    },
    /// Commands for interacting with Sequencer block heights
    #[clap(name = "blockheight")]
    BlockHeight {
        #[clap(subcommand)]
        command: BlockHeightCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    /// Create a new Sequencer account
    Create,
}

#[derive(Debug, Subcommand)]
pub enum BalanceCommand {
    /// Get the balance of a Sequencer account
    Get(BalanceGetArgs),
}

#[derive(Args, Debug)]
pub struct BalanceGetArgs {
    /// The url of the Sequencer node
    #[clap(long)]
    pub(crate) sequencer_url: String,
    /// The address of the Sequencer account
    pub(crate) address: SequencerAddressArg,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SequencerAddressArg(pub(crate) Address);

impl FromStr for SequencerAddressArg {
    type Err = eyre::Report;

    /// Parse a string into a Sequencer Address
    fn from_str(s: &str) -> eyre::Result<Self, Self::Err> {
        let address_bytes = hex::decode(s).wrap_err(
            "failed to decode address. address should be 20 bytes long. do not prefix with 0x",
        )?;
        let address =
            Address::try_from_slice(address_bytes.as_ref()).wrap_err("failed to create address")?;

        Ok(Self(address))
    }
}

#[derive(Debug, Subcommand)]
pub enum BlockHeightCommand {
    /// Get the current block height of the Sequencer node
    Get(BlockHeightGetArgs),
}

#[derive(Args, Debug)]
pub struct BlockHeightGetArgs {
    /// The url of the Sequencer node
    #[clap(long)]
    pub(crate) sequencer_url: String,
}
