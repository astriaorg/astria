use color_eyre::eyre;
use serde::{
    Deserialize,
    Serialize,
};

use crate::cli::rollup::{
    ConfigCreateArgs,
    GenesisAccountArg,
};

/// Rollup contains the deployment config for a rollup
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rollup {
    pub(crate) namespace: String,
    #[serde(rename = "config")]
    pub(crate) deployment_config: RollupDeploymentConfig,
    pub(crate) ingress: IngressConfig,
}

impl TryFrom<&ConfigCreateArgs> for Rollup {
    type Error = eyre::Report;

    fn try_from(args: &ConfigCreateArgs) -> eyre::Result<Self> {
        let deployment_config = RollupDeploymentConfig::try_from(args)?;
        let ingress = IngressConfig::from(args);

        Ok(Self {
            namespace: args.namespace.clone(),
            deployment_config,
            ingress,
        })
    }
}

impl TryInto<String> for Rollup {
    type Error = eyre::Report;

    /// Serializes Rollup to a yaml string
    fn try_into(self) -> eyre::Result<String> {
        let yaml_str = serde_yaml::to_string(&self)?;
        Ok(yaml_str)
    }
}

/// Describes a rollup deployment config. Serializes to a yaml file for usage with Helm,
/// thus the `rename_all = "camelCase"` naming convention.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollupDeploymentConfig {
    #[serde(rename = "useTTY")]
    use_tty: bool,
    log_level: String,
    rollup: RollupConfig,
    sequencer: SequencerConfig,
}

impl RollupDeploymentConfig {
    #[must_use]
    pub fn get_filename(&self) -> String {
        format!("{}-rollup-conf.yaml", self.rollup.name)
    }

    #[must_use]
    pub fn get_chart_release_name(&self) -> String {
        format!("{}-rollup", self.rollup.name)
    }

    #[must_use]
    pub fn get_rollup_name(&self) -> String {
        self.rollup.name.clone()
    }

    #[must_use]
    pub fn get_initial_sequencer_height(&self) -> u64 {
        self.sequencer.initial_block_height
    }

    pub fn set_initial_sequencer_height(&mut self, new_height: u64) {
        self.sequencer.initial_block_height = new_height;
    }
}

impl From<&ConfigCreateArgs> for IngressConfig {
    fn from(args: &ConfigCreateArgs) -> Self {
        Self {
            hostname: args.hostname.clone(),
        }
    }
}

impl TryFrom<&ConfigCreateArgs> for RollupDeploymentConfig {
    type Error = eyre::Report;

    fn try_from(args: &ConfigCreateArgs) -> eyre::Result<Self> {
        let chain_id = args
            .chain_id
            .clone()
            .unwrap_or(format!("{}-chain", args.name));

        // Set to block 1 if nothing set.
        let sequencer_initial_block_height = args.sequencer_initial_block_height.unwrap_or(1);

        let genesis_accounts = args
            .genesis_accounts
            .clone()
            .into_iter()
            .map(GenesisAccount::from)
            .collect();

        Ok(Self {
            use_tty: args.use_tty,
            log_level: args.log_level.clone(),
            rollup: RollupConfig {
                name: args.name.clone(),
                chain_id,
                network_id: args.network_id.to_string(),
                skip_empty_blocks: args.skip_empty_blocks,
                genesis_accounts,
            },
            sequencer: SequencerConfig {
                initial_block_height: sequencer_initial_block_height,
                websocket: args.sequencer_websocket.clone(),
                rpc: args.sequencer_rpc.clone(),
            },
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollupConfig {
    name: String,
    chain_id: String,
    // NOTE - String here because yaml will serialize large ints w/ scientific notation
    network_id: String,
    skip_empty_blocks: bool,
    genesis_accounts: Vec<GenesisAccount>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenesisAccount {
    address: String,
    // NOTE - string because yaml will serialize large ints w/ scientific notation
    balance: String,
}

impl From<GenesisAccountArg> for GenesisAccount {
    fn from(arg: GenesisAccountArg) -> Self {
        Self {
            address: arg.address,
            balance: arg.balance.to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SequencerConfig {
    initial_block_height: u64,
    websocket: String,
    rpc: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngressConfig {
    hostname: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::rollup::{
        ConfigCreateArgs,
        GenesisAccountArg,
    };

    #[test]
    fn test_from_all_cli_args() -> eyre::Result<()> {
        // Case 1: All args provided
        let args = ConfigCreateArgs {
            use_tty: true,
            log_level: "debug".to_string(),
            name: "rollup1".to_string(),
            chain_id: Some("chain1".to_string()),
            network_id: 1,
            skip_empty_blocks: true,
            genesis_accounts: vec![
                GenesisAccountArg {
                    address: "0xA5TR14".to_string(),
                    balance: 1_000_000_000_000_000_000,
                },
                GenesisAccountArg {
                    address: "0x420XYZ69".to_string(),
                    balance: 420,
                },
            ],
            sequencer_initial_block_height: Some(10),
            sequencer_websocket: "ws://localhost:8080".to_string(),
            sequencer_rpc: "http://localhost:8081".to_string(),
            hostname: "test.com".to_string(),
            namespace: "test-cluster".to_string(),
        };

        let expected_config = Rollup {
            namespace: "test-cluster".to_string(),
            deployment_config: RollupDeploymentConfig {
                use_tty: true,
                log_level: "debug".to_string(),
                rollup: RollupConfig {
                    name: "rollup1".to_string(),
                    chain_id: "chain1".to_string(),
                    network_id: "1".to_string(),
                    skip_empty_blocks: true,
                    genesis_accounts: vec![
                        GenesisAccount {
                            address: "0xA5TR14".to_string(),
                            balance: "1000000000000000000".to_string(),
                        },
                        GenesisAccount {
                            address: "0x420XYZ69".to_string(),
                            balance: "420".to_string(),
                        },
                    ],
                },
                sequencer: SequencerConfig {
                    initial_block_height: 10,
                    websocket: "ws://localhost:8080".to_string(),
                    rpc: "http://localhost:8081".to_string(),
                },
            },
            ingress: IngressConfig {
                hostname: "test.com".to_string(),
            },
        };

        let result = Rollup::try_from(&args)?;
        assert_eq!(result, expected_config);

        Ok(())
    }

    #[test]
    fn test_from_minimum_cli_args() -> eyre::Result<()> {
        // No `Option` wrapped args provided. Tests defaults that are decided
        //  explicitly in the `try_from` impl.
        // NOTE - there are some defaults that are handled in the arg struct,
        //  like the sequencer ws and rpc urls, so we still must pass them in here.
        let args = ConfigCreateArgs {
            use_tty: false,
            log_level: "info".to_string(),
            name: "rollup2".to_string(),
            chain_id: None,
            network_id: 2_211_011_801,
            skip_empty_blocks: false,
            genesis_accounts: vec![GenesisAccountArg {
                address: "0xA5TR14".to_string(),
                balance: 10000,
            }],
            sequencer_initial_block_height: None,
            sequencer_websocket: "ws://localhost:8082".to_string(),
            sequencer_rpc: "http://localhost:8083".to_string(),
            hostname: "localdev.me".to_string(),
            namespace: "astria-dev-cluster".to_string(),
        };

        let expected_config = Rollup {
            namespace: "astria-dev-cluster".to_string(),
            deployment_config: RollupDeploymentConfig {
                use_tty: false,
                log_level: "info".to_string(),
                rollup: RollupConfig {
                    name: "rollup2".to_string(),
                    chain_id: "rollup2-chain".to_string(), // Derived from name
                    network_id: "2211011801".to_string(),
                    skip_empty_blocks: false,
                    genesis_accounts: vec![GenesisAccount {
                        address: "0xA5TR14".to_string(),
                        balance: "10000".to_string(),
                    }],
                },
                sequencer: SequencerConfig {
                    initial_block_height: 1, // Default value
                    websocket: "ws://localhost:8082".to_string(),
                    rpc: "http://localhost:8083".to_string(),
                },
            },
            ingress: IngressConfig {
                hostname: "localdev.me".to_string(),
            },
        };

        let result = Rollup::try_from(&args)?;
        assert_eq!(result, expected_config);

        Ok(())
    }
}
