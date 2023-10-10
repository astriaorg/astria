use std::str::FromStr;

use clap::{
    Args,
    Subcommand,
};
use color_eyre::eyre;
use serde::Serialize;

/// Check if a string is a valid balance.
fn is_valid_balance(s: &str) -> bool {
    s.parse::<u128>().is_ok()
}

/// Manage your rollups
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage your rollup configs
    Config {
        #[clap(subcommand)]
        command: ConfigCommand,
    },
    /// Manage your rollup deployments
    Deployment {
        #[clap(subcommand)]
        command: DeploymentCommand,
    },
}

/// Commands for managing rollup configs.
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Create a new rollup config
    Create(ConfigCreateArgs),
    /// Edit a rollup config
    Edit(ConfigEditArgs),
    /// Delete a rollup config
    Delete(ConfigDeleteArgs),
}

#[derive(Args, Debug, Serialize)]
pub struct ConfigCreateArgs {
    #[clap(long, env = "ROLLUP_USE_TTY")]
    pub use_tty: bool,
    #[clap(long, env = "ROLLUP_LOG_LEVEL")]
    pub log_level: String,

    // rollup config
    #[clap(long = "rollup.name", env = "ROLLUP_NAME")]
    /// The name of the rollup
    pub name: String,
    #[clap(long = "rollup.chain-id", env = "ROLLUP_CHAIN_ID", required = false)]
    /// Optional. Will be derived from the rollup name if not provided
    pub chain_id: Option<String>,
    #[clap(long = "rollup.network-id", env = "ROLLUP_NETWORK_ID")]
    pub network_id: u64,
    #[clap(long = "rollup.skip-empty-blocks", env = "ROLLUP_SKIP_EMPTY_BLOCKS")]
    pub skip_empty_blocks: bool,

    #[clap(long = "rollup.genesis-accounts", env = "ROLLUP_GENESIS_ACCOUNTS", num_args = 1..)]
    pub genesis_accounts: Vec<GenesisAccountArg>,

    // sequencer config
    #[clap(
        long = "sequencer.initial-block-height",
        env = "ROLLUP_SEQUENCER_INITIAL_BLOCK_HEIGHT",
        required = false
    )]
    /// Optional. If not set, will be determined from the current block height of the sequencer
    pub sequencer_initial_block_height: Option<u64>,
    #[clap(long = "sequencer.websocket", env = "ROLLUP_SEQUENCER_WEBSOCKET")]
    pub sequencer_websocket: String,
    #[clap(long = "sequencer.rpc", env = "ROLLUP_SEQUENCER_RPC")]
    pub sequencer_rpc: String,

    // celestia config
    #[clap(long = "celestia.full-node-url", env = "ROLLUP_CELESTIA_FULL_NODE_URL")]
    pub celestia_full_node_url: String,
}

/// `GenesisAccount` is a struct that represents a genesis account to be funded.
/// It has the form of `address:balance`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GenesisAccountArg {
    pub address: String,
    pub balance: String,
}

impl FromStr for GenesisAccountArg {
    type Err = eyre::Report;

    /// Parse a string of the form `address:balance` into a `GenesisAccountArg`.
    /// If the balance is not provided, it will default to 0.
    ///
    /// # Errors
    ///
    /// * If the address is missing
    /// * If the address is empty
    fn from_str(s: &str) -> eyre::Result<Self, Self::Err> {
        let mut parts = s.splitn(2, ':');

        let address = parts
            .next()
            .ok_or_else(|| eyre::eyre!("Missing address"))?
            .to_string();
        if address.is_empty() {
            return Err(eyre::eyre!("Empty address"));
        }

        let balance_str = parts.next().unwrap_or("0");
        if !is_valid_balance(balance_str) {
            return Err(eyre::eyre!("Invalid balance"));
        }
        let balance = balance_str.to_string();

        Ok(GenesisAccountArg {
            address,
            balance,
        })
    }
}

#[derive(Args, Debug)]
pub struct ConfigEditArgs {
    /// The filepath of the config to edit
    #[clap(long = "config")]
    pub(crate) config_path: String,
    /// The key of the field to edit. Accepts dot notated yaml path.
    pub(crate) key: String,
    /// The value to set the field to
    pub(crate) value: String,
}

#[derive(Args, Debug)]
pub struct ConfigDeleteArgs {
    /// The filepath of the config to delete
    #[clap(long = "config")]
    pub(crate) config_path: String,
}

#[derive(Debug, Subcommand)]
pub enum DeploymentCommand {
    /// Deploy a rollup
    Create(DeploymentCreateArgs),
    /// Delete a rollup
    Delete(DeploymentDeleteArgs),
    /// List all deployed rollups
    List,
}

#[derive(Args, Debug, Serialize)]
pub struct DeploymentCreateArgs {
    /// The filepath of the config to deploy
    #[clap(long = "config")]
    pub(crate) config_path: String,
    /// The faucet private key
    #[clap(long, env = "FAUCET_PRIVATE_KEY")]
    pub(crate) faucet_private_key: String,
    /// The sequencer private key
    #[clap(long, env = "SEQUENCER_PRIVATE_KEY")]
    pub(crate) sequencer_private_key: String,
}

#[derive(Args, Debug)]
pub struct DeploymentDeleteArgs {
    /// The filepath of the config to delete
    #[clap(long = "config")]
    pub(crate) config_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_with_balance() {
        let input = "0x1234abcd:1000";
        let expected = GenesisAccountArg {
            address: "0x1234abcd".to_string(),
            balance: "1000".to_string(),
        };
        let result: GenesisAccountArg = input.parse().unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_from_str_without_balance() {
        let input = "0x1234abcd";
        let expected = GenesisAccountArg {
            address: "0x1234abcd".to_string(),
            balance: "0".to_string(),
        };
        let result: GenesisAccountArg = input.parse().unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_from_str_invalid_balance() {
        let input = "0x1234abcd:invalid_balance";
        let result: Result<GenesisAccountArg, _> = input.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_is_valid_balance() {
        assert!(is_valid_balance("100"));
        assert!(!is_valid_balance("not_a_number"));
        assert!(is_valid_balance("0"));
    }
}
