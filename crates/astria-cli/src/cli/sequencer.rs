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
    /// Commands requiring authority for Sequencer
    Sudo {
        #[clap(subcommand)]
        command: SudoCommand,
    },
    /// Command for sending balance between accounts
    Transfer(TransferArgs),
    /// Command for initializing a bridge account
    InitBridgeAccount(InitBridgeAccountArgs),
    /// Command for transferring to a bridge account
    BridgeLock(BridgeLockArgs),
}

#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    /// Create a new Sequencer account
    Create,
    Balance(BasicAccountArgs),
    Nonce(BasicAccountArgs),
}

#[derive(Debug, Subcommand)]
pub enum BalanceCommand {
    /// Get the balance of a Sequencer account
    Get(BasicAccountArgs),
}

#[derive(Debug, Subcommand)]
pub enum SudoCommand {
    IbcRelayer {
        #[clap(subcommand)]
        command: IbcRelayerChangeCommand,
    },
    FeeAsset {
        #[clap(subcommand)]
        command: FeeAssetChangeCommand,
    },
    Mint(MintArgs),
    SudoAddressChange(SudoAddressChangeArgs),
    ValidatorUpdate(ValidatorUpdateArgs),
}

#[derive(Debug, Subcommand)]
pub enum IbcRelayerChangeCommand {
    /// Add IBC Relayer
    Add(IbcRelayerChangeArgs),
    /// Remove IBC Relayer
    Remove(IbcRelayerChangeArgs),
}

#[derive(Debug, Subcommand)]
pub enum FeeAssetChangeCommand {
    /// Add Fee Asset
    Add(FeeAssetChangeArgs),
    /// Remove Fee ASset
    Remove(FeeAssetChangeArgs),
}

#[derive(Args, Debug)]
pub struct BasicAccountArgs {
    /// The url of the Sequencer node
    #[clap(
        long,
        env = "SEQUENCER_URL", 
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[clap(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// The address of the Sequencer account
    pub(crate) address: SequencerAddressArg,
}

#[derive(Args, Debug)]
pub struct TransferArgs {
    // The address of the Sequencer account to send amount to
    pub(crate) to_address: SequencerAddressArg,
    // The amount being sent
    #[clap(long)]
    pub(crate) amount: u128,
    /// The private key of account being sent from
    #[clap(long, env = "SEQUENCER_PRIVATE_KEY")]
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[clap(
        long,
        env = "SEQUENCER_URL", 
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[clap(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
}

#[derive(Args, Debug)]
pub struct FeeAssetChangeArgs {
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[clap(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[clap(
        long,
        env = "SEQUENCER_URL", 
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[clap(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// Asset's denomination string
    #[clap(long)]
    pub(crate) asset: String,
}

#[derive(Args, Debug)]
pub struct IbcRelayerChangeArgs {
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[clap(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[clap(
        long,
        env = "SEQUENCER_URL", 
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[clap(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// The address to add or remove as an IBC relayer
    #[clap(long)]
    pub(crate) address: SequencerAddressArg,
}

#[derive(Args, Debug)]
pub struct InitBridgeAccountArgs {
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[clap(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[clap(
        long,
        env = "SEQUENCER_URL", 
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[clap(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// Plaintext rollup name (to be hashed into a rollup ID)
    /// to initialize the bridge account with.
    #[clap(long)]
    pub(crate) rollup_name: String,
}

#[derive(Args, Debug)]
pub struct BridgeLockArgs {
    /// The address of the Sequencer account to lock amount to
    pub(crate) to_address: SequencerAddressArg,
    /// The amount being locked
    #[clap(long)]
    pub(crate) amount: u128,
    #[clap(long)]
    pub(crate) destination_chain_address: String,
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[clap(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[clap(
        long,
        env = "SEQUENCER_URL", 
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[clap(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
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
    #[clap(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[clap(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
}

#[derive(Args, Debug)]
pub struct MintArgs {
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[clap(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[clap(
        long,
        env = "SEQUENCER_URL", 
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[clap(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// The address to mint to
    #[clap(long)]
    pub(crate) to_address: SequencerAddressArg,
    /// The amount to mint
    #[clap(long)]
    pub(crate) amount: u128,
}

#[derive(Args, Debug)]
pub struct SudoAddressChangeArgs {
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[clap(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[clap(
        long,
        env = "SEQUENCER_URL", 
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[clap(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// The new address to take over sudo privileges
    #[clap(long)]
    pub(crate) address: SequencerAddressArg,
}

#[derive(Args, Debug)]
pub struct ValidatorUpdateArgs {
    /// The url of the Sequencer node
    #[clap(
        long,
        env = "SEQUENCER_URL", 
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[clap(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// The private key of the sudo account authorizing change
    #[clap(long, env = "SEQUENCER_PRIVATE_KEY")]
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    pub(crate) private_key: String,
    /// The address of the Validator being updated
    #[clap(long)]
    pub(crate) validator_public_key: String,
    /// The power the validator is being updated to
    #[clap(long)]
    pub(crate) power: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequencer_address_arg_from_str_valid() {
        let hex_str = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0";
        let bytes = hex::decode(hex_str).unwrap();
        let expected_address = Address::try_from_slice(&bytes).unwrap();

        let sequencer_address_arg: SequencerAddressArg = hex_str.parse().unwrap();
        assert_eq!(sequencer_address_arg, SequencerAddressArg(expected_address));
    }

    #[test]
    fn test_sequencer_address_arg_from_str_invalid() {
        let hex_str = "invalidhexstr";
        let result: eyre::Result<SequencerAddressArg> = hex_str.parse();
        assert!(result.is_err());

        let error_message = format!("{:?}", result.unwrap_err());
        assert!(error_message.contains("failed to decode address"));
    }
}
