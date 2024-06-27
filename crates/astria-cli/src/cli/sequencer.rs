use astria_sequencer_client::Address;
use clap::{
    Args,
    Subcommand,
};

/// Interact with a Sequencer node
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Commands for interacting with Sequencer accounts
    Account {
        #[command(subcommand)]
        command: AccountCommand,
    },
    /// Utilities for constructing and inspecting sequencer addresses
    Address {
        #[command(subcommand)]
        command: AddressCommand,
    },
    /// Commands for interacting with Sequencer balances
    Balance {
        #[command(subcommand)]
        command: BalanceCommand,
    },
    /// Commands for interacting with Sequencer block heights
    #[command(name = "blockheight")]
    BlockHeight {
        #[command(subcommand)]
        command: BlockHeightCommand,
    },
    /// Commands requiring authority for Sequencer
    Sudo {
        #[command(subcommand)]
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
pub enum AddressCommand {
    /// Construct a bech32m Sequencer address given a public key
    Bech32m(Bech32mAddressArgs),
}

#[derive(Debug, Subcommand)]
pub enum BalanceCommand {
    /// Get the balance of a Sequencer account
    Get(BasicAccountArgs),
}

#[derive(Debug, Subcommand)]
pub enum SudoCommand {
    IbcRelayer {
        #[command(subcommand)]
        command: IbcRelayerChangeCommand,
    },
    FeeAsset {
        #[command(subcommand)]
        command: FeeAssetChangeCommand,
    },
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
    /// Remove Fee Asset
    Remove(FeeAssetChangeArgs),
}

#[derive(Args, Debug)]
pub struct BasicAccountArgs {
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The address of the Sequencer account
    pub(crate) address: Address,
}

#[derive(Args, Debug)]
pub struct Bech32mAddressArgs {
    /// The hex formatted byte part of the bech32m address
    #[arg(long)]
    pub(crate) bytes: String,
    /// The human readable prefix (Hrp) of the bech32m adress
    #[arg(long, default_value = "astria")]
    pub(crate) prefix: String,
}

#[derive(Args, Debug)]
pub struct TransferArgs {
    // The address of the Sequencer account to send amount to
    pub(crate) to_address: Address,
    // The amount being sent
    #[arg(long)]
    pub(crate) amount: u128,
    /// The bech32m prefix that will be used for constructing addresses using the private key
    #[arg(long, default_value = "astria")]
    pub(crate) prefix: String,
    /// The private key of account being sent from
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
}

#[derive(Args, Debug)]
pub struct FeeAssetChangeArgs {
    /// The bech32m prefix that will be used for constructing addresses using the private key
    #[arg(long, default_value = "astria")]
    pub(crate) prefix: String,
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// Asset's denomination string
    #[arg(long)]
    pub(crate) asset: String,
}

#[derive(Args, Debug)]
pub struct IbcRelayerChangeArgs {
    /// The prefix to construct a bech32m address given the private key.
    #[arg(long, default_value = "astria")]
    pub(crate) prefix: String,
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// The address to add or remove as an IBC relayer
    #[arg(long)]
    pub(crate) address: Address,
}

#[derive(Args, Debug)]
pub struct InitBridgeAccountArgs {
    /// The bech32m prefix that will be used for constructing addresses using the private key
    #[arg(long, default_value = "astria")]
    pub(crate) prefix: String,
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// Plaintext rollup name (to be hashed into a rollup ID)
    /// to initialize the bridge account with.
    #[arg(long)]
    pub(crate) rollup_name: String,
}

#[derive(Args, Debug)]
pub struct BridgeLockArgs {
    /// The address of the Sequencer account to lock amount to
    pub(crate) to_address: Address,
    /// The amount being locked
    #[arg(long)]
    pub(crate) amount: u128,
    #[arg(long)]
    pub(crate) destination_chain_address: String,
    /// The prefix to construct a bech32m address given the private key.
    #[arg(long, default_value = "astria")]
    pub(crate) prefix: String,
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
}

#[derive(Debug, Subcommand)]
pub enum BlockHeightCommand {
    /// Get the current block height of the Sequencer node
    Get(BlockHeightGetArgs),
}

#[derive(Args, Debug)]
pub struct BlockHeightGetArgs {
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
}

#[derive(Args, Debug)]
pub struct SudoAddressChangeArgs {
    /// The bech32m prefix that will be used for constructing addresses using the private key
    #[arg(long, default_value = "astria")]
    pub(crate) prefix: String,
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) private_key: String,
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// The new address to take over sudo privileges
    #[arg(long)]
    pub(crate) address: Address,
}

#[derive(Args, Debug)]
pub struct ValidatorUpdateArgs {
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub(crate) sequencer_url: String,
    /// The chain id of the sequencing chain being used
    #[arg(
        long = "sequencer.chain-id",
        env = "ROLLUP_SEQUENCER_CHAIN_ID",
        default_value = crate::cli::DEFAULT_SEQUENCER_CHAIN_ID
    )]
    pub sequencer_chain_id: String,
    /// The bech32m prefix that will be used for constructing addresses using the private key
    #[arg(long, default_value = "astria")]
    pub(crate) prefix: String,
    /// The private key of the sudo account authorizing change
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    pub(crate) private_key: String,
    /// The address of the Validator being updated
    #[arg(long)]
    pub(crate) validator_public_key: String,
    /// The power the validator is being updated to
    #[arg(long)]
    pub(crate) power: u32,
}
