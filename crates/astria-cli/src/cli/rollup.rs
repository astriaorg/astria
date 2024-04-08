use std::str::FromStr;

use clap::{
    Args,
    Subcommand,
};
use color_eyre::eyre;
use serde::Serialize;

const DEFAULT_ROLLUP_CHART_PATH: &str =
    "https://github.com/astriaorg/charts/releases/download/evm-rollup-0.11.1/evm-rollup-0.11.1.tgz";
const DEFAULT_SEQUENCER_GRPC: &str = "https://grpc.sequencer.dusk-4.devnet.astria.org/";
const DEFAULT_LOG_LEVEL: &str = "debug";
const DEFAULT_NETWORK_ID: u64 = 1337;
const DEFAULT_EXECUTION_COMMIT_LEVEL: &str = "SoftOnly";
const DEFAULT_HOSTNAME: &str = "localdev.me";
const DEFAULT_NAMESPACE: &str = "astria-dev-cluster";
const DEFAULT_ROLLUP_GENESIS_BRIDGE_ALLOWED_ASSET_DENOM: &str = "nria";

/// Remove the 0x prefix from a hex string if present
fn strip_0x_prefix(s: &str) -> &str {
    if let Some(stripped) = s.strip_prefix("0x") {
        stripped
    } else {
        s
    }
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

#[derive(Args, Debug, Serialize, Clone)]
pub struct ConfigCreateArgs {
    #[clap(long, env = "ROLLUP_USE_TTY", default_value = "true")]
    pub use_tty: bool,
    #[clap(long, env = "ROLLUP_LOG_LEVEL", default_value = DEFAULT_LOG_LEVEL)]
    pub log_level: String,

    // rollup config
    /// The name of the rollup
    #[clap(long = "rollup.name", env = "ROLLUP_NAME")]
    pub name: String,
    /// The Network ID for the EVM chain
    #[clap(long = "rollup.network-id", env = "ROLLUP_NETWORK_ID", default_value_t = DEFAULT_NETWORK_ID)]
    pub network_id: u64,
    /// The Execution Commit level
    #[clap(
        long = "rollup.execution-commit-level",
        env = "ROLLUP_EXECUTION_COMMIT_LEVEL",
        default_value = DEFAULT_EXECUTION_COMMIT_LEVEL
    )]
    pub execution_commit_level: String,
    /// Choose to allow genesis extra data override
    #[clap(
        long = "rollup.override-genesis-extra-data",
        env = "ROLLUP_OVERRIDE_GENESIS_EXTRA_DATA"
    )]
    pub override_genesis_extra_data: bool,
    /// Optional. If set, will be used as the bridge address. If not set, nothing happens.
    #[clap(long = "rollup.bridge-address", env = "ROLLUP_BRIDGE_ADDRESS")]
    pub bridge_address: Option<String>,
    /// The allowed asset denom for the bridge
    #[clap(
        long = "rollup.bridge-allowed-asset-denom",
        env = "ROLLUP_BRIDGE_ALLOWED_ASSET_DENOM",
        default_value = DEFAULT_ROLLUP_GENESIS_BRIDGE_ALLOWED_ASSET_DENOM
    )]
    pub bridge_allowed_asset_denom: String,
    /// List of genesis accounts to fund, in the form of `address:balance`
    #[clap(
        long = "rollup.genesis-accounts", 
        env = "ROLLUP_GENESIS_ACCOUNTS", 
        num_args = 1..,
        value_delimiter = ','
    )]
    pub genesis_accounts: Vec<GenesisAccountArg>,
    // sequencer config
    /// Optional. If not set, will be determined from the current block height of the sequencer
    #[clap(
        long = "sequencer.initial-block-height",
        env = "ROLLUP_SEQUENCER_INITIAL_BLOCK_HEIGHT"
    )]
    #[arg(value_parser = validate_initial_block_height)]
    pub sequencer_initial_block_height: Option<u64>,
    /// Optional. If not set, will default to the devnet sequencer gRPC address
    #[clap(
        long = "sequencer.grpc", 
        env = "ROLLUP_SEQUENCER_GRPC", 
        default_value = DEFAULT_SEQUENCER_GRPC
    )]
    pub sequencer_grpc: String,
    /// Optional. If not set, will be default to the devnet sequencer rpc address
    #[clap(
        long = "sequencer.rpc", 
        env = "ROLLUP_SEQUENCER_RPC", 
        default_value = crate::cli::DEFAULT_SEQUENCER_RPC
    )]
    pub sequencer_rpc: String,
    /// Optional. Will default to 'localdev.me' for local deployments. Will need to separately
    /// configure other hosts
    #[clap(
        long = "hostname", 
        env = "ROLLUP_HOSTNAME", 
        default_value = DEFAULT_HOSTNAME
    )]
    pub hostname: String,
    /// Configures the k8s namespace rollup will be deployed to
    #[clap(long, env = "ROLLUP_NAMESPACE", default_value = DEFAULT_NAMESPACE)]
    pub namespace: String,
    /// Choose to enable the Celestia feature
    #[clap(long = "celestia-node.enabled", env = "ROLLUP_ENABLE_CELESTIA_NODE")]
    pub enable_celestia_node: bool,
}

fn validate_initial_block_height(val: &str) -> Result<u64, String> {
    match val.parse::<u64>() {
        Ok(height) if height >= 2 => Ok(height),
        _ => Err(String::from("The block height must be at least 2.")),
    }
}

/// `GenesisAccountArg` is a struct that represents a genesis account to be funded.
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
    /// * If the address is not a valid hex string that decodes to 20 bytes
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

        let balance_str = parts.next().unwrap_or("1000000000000000000");
        let balance = balance_str
            .parse::<u128>()
            .map_err(|e| eyre::eyre!("Invalid balance. Could not parse to u128: {}", e))?;

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
    #[clap(
        long,
        env = "ROLLUP_CHART_PATH",
        default_value = DEFAULT_ROLLUP_CHART_PATH
    )]
    pub(crate) chart_path: String,
    /// Set if you want to do a dry run of the deployment
    #[clap(long, env = "ROLLUP_DRY_RUN", default_value = "false")]
    pub(crate) dry_run: bool,
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    /// Faucet private key
    #[clap(long, env = "ROLLUP_FAUCET_PRIVATE_KEY")]
    pub(crate) faucet_private_key: String,
    /// Sequencer private key
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    #[clap(long, env = "ROLLUP_SEQUENCER_PRIVATE_KEY")]
    pub(crate) sequencer_private_key: String,
    /// Set if you want to see all k8s resources created by the deployment
    /// Set if you want to do a dry run of the deployment
    #[clap(long, env = "ROLLUP_DEBUG_DEPLOY", default_value = "false")]
    pub(crate) debug: bool,
}

#[derive(Args, Debug)]
pub struct DeploymentDeleteArgs {
    /// The filepath of the target deployment's config
    #[clap(long = "config")]
    pub(crate) config_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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
            balance: 1_000_000_000_000_000_000,
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
}
