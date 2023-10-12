use std::str::FromStr;

use clap::{
    Args,
    Subcommand,
};
use color_eyre::eyre;
use serde::Serialize;

/// Remove the 0x prefix from a hex string if present
fn strip_0x_prefix(s: &str) -> &str {
    if s.starts_with("0x") { &s[2..] } else { s }
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
    pub balance: u128,
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
    /// * If the address is not a valid im
    fn from_str(s: &str) -> eyre::Result<Self, Self::Err> {
        let mut parts = s.splitn(2, ':');

        let address = parts.next().ok_or_else(|| eyre::eyre!("Missing address"))?;
        let address = strip_0x_prefix(address).to_string();
        if address.is_empty() {
            return Err(eyre::eyre!("Empty address"));
        }
        let decoded =
            hex::decode(&address).map_err(|e| eyre::eyre!("Invalid hex address: {}", e))?;
        if decoded.len() != 20 {
            return Err(eyre::eyre!(
                "Address must be a 20-byte hex string, or 40 characters."
            ));
        }

        let balance_str = parts.next().unwrap_or("0");
        let balance = balance_str
            .parse::<u128>()
            .map_err(|e| eyre::eyre!("Invalid balance: {}", e))?;

        Ok(GenesisAccountArg {
            address,
            balance,
        })
    }
}

#[derive(Args, Debug)]
pub struct ConfigEditArgs {
    /// The filepath of the config to edit
    #[clap(long = "config", env = "ROLLUP_CONFIG_PATH")]
    pub(crate) config_path: String,
    /// The key of the field to edit. Accepts dot notated yaml path.
    pub(crate) key: String,
    /// The value to set the field to
    pub(crate) value: String,
}

#[derive(Args, Debug)]
pub struct ConfigDeleteArgs {
    /// The filepath of the config to delete
    #[clap(long = "config", env = "ROLLUP_CONFIG_PATH")]
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
    /// Filepath of the config to deploy
    #[clap(long = "config", env = "ROLLUP_CONFIG_PATH")]
    pub(crate) config_path: String,
    /// Optional path to a rollup chart that can override the default remote helm chart
    #[clap(long, env = "ROLLUP_CHART_PATH")]
    pub(crate) chart_path: Option<String>,
    /// Set if you want to do a dry run of the deployment
    #[clap(long, env = "ROLLUP_DRY_RUN", default_value = "false")]
    pub(crate) dry_run: bool,
    /// Faucet private key
    #[clap(long, env = "ROLLUP_FAUCET_PRIVATE_KEY")]
    pub(crate) faucet_private_key: String,
    /// Sequencer private key
    #[clap(long, env = "ROLLUP_SEQUENCER_PRIVATE_KEY")]
    pub(crate) sequencer_private_key: String,
}

#[derive(Args, Debug)]
pub struct DeploymentDeleteArgs {
    /// The filepath of the target deployment's config
    #[clap(long = "config")]
    pub(crate) config_path: String,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use assert_cmd;

    use super::*;
    use crate::test_utils::with_temp_directory;

    #[test]
    fn test_genesis_account_arg_from_str_with_balance() {
        let input = "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30:1000";
        let expected = GenesisAccountArg {
            address: "aC21B97d35Bf75A7dAb16f35b111a50e78A72F30".to_string(),
            balance: 1000,
        };
        let result: GenesisAccountArg = input.parse().unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_genesis_account_arg_from_str_without_balance() {
        let input = "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30";
        let expected = GenesisAccountArg {
            address: "aC21B97d35Bf75A7dAb16f35b111a50e78A72F30".to_string(),
            balance: 0,
        };
        let result: GenesisAccountArg = input.parse().unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_genesis_account_arg_from_str_invalid_balance() {
        let input = "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30:invalid_balance";
        let result: Result<GenesisAccountArg, _> = input.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_genesis_account_arg_from_str_invalid_address() {
        let input = "0x1234abcd:1000";
        let result: Result<GenesisAccountArg, _> = input.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_genesis_account_arg_from_str_no_0x_prefix() {
        let input = "aC21B97d35Bf75A7dAb16f35b111a50e78A72F30:1000";
        let expected = GenesisAccountArg {
            address: "aC21B97d35Bf75A7dAb16f35b111a50e78A72F30".to_string(),
            balance: 1000,
        };
        let result: GenesisAccountArg = input.parse().unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_envars_are_parsed_for_config_create() {
        with_temp_directory(|_dir| {
            let mut cmd = assert_cmd::Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

            cmd.arg("rollup")
                .arg("config")
                .arg("create")
                .env("ROLLUP_USE_TTY", "true")
                .env("ROLLUP_LOG_LEVEL", "debug")
                .env("ROLLUP_NAME", "testtest")
                .env("ROLLUP_CHAIN_ID", "test_chain_id")
                .env("ROLLUP_NETWORK_ID", "53")
                .env("ROLLUP_SKIP_EMPTY_BLOCKS", "true")
                .env("ROLLUP_GENESIS_ACCOUNTS", "0x1234abcd:1000")
                .env("ROLLUP_SEQUENCER_INITIAL_BLOCK_HEIGHT", "10")
                .env("ROLLUP_SEQUENCER_WEBSOCKET", "ws://localhost:8080")
                .env("ROLLUP_SEQUENCER_RPC", "http://localhost:8081")
                .env("ROLLUP_CELESTIA_FULL_NODE_URL", "http://celestia_node")
                .assert()
                .success();

            assert!(PathBuf::from("testtest-rollup-conf.yaml").exists());
        });
    }

    #[test]
    fn test_error_when_incorrect_envar_values() {
        with_temp_directory(|_dir| {
            let mut cmd = assert_cmd::Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

            cmd.arg("rollup")
                .arg("config")
                .arg("create")
                .env("ROLLUP_USE_TTY", "not_a_bool")
                .env("ROLLUP_LOG_LEVEL", "debug")
                .env("ROLLUP_NAME", "testtest")
                .env("ROLLUP_CHAIN_ID", "test_chain_id")
                .env("ROLLUP_NETWORK_ID", "not_a_number")
                .env("ROLLUP_SKIP_EMPTY_BLOCKS", "true")
                .env("ROLLUP_GENESIS_ACCOUNTS", "0x1234abcd:1000")
                .env("ROLLUP_SEQUENCER_INITIAL_BLOCK_HEIGHT", "10")
                .env("ROLLUP_SEQUENCER_WEBSOCKET", "ws://localhost:8080")
                .env("ROLLUP_SEQUENCER_RPC", "http://localhost:8081")
                .env("ROLLUP_CELESTIA_FULL_NODE_URL", "http://celestia_node")
                .assert()
                .failure();
        });
    }
}
