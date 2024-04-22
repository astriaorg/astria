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
    #[serde(rename = "global")]
    pub(crate) globals_config: GlobalsConfig,
    #[serde(rename = "config")]
    pub(crate) deployment_config: RollupDeploymentConfig,
    #[serde(rename = "ingress")]
    pub(crate) ingress_config: IngressConfig,
    #[serde(rename = "celestia-node")]
    pub(crate) celestia_node: CelestiaNode,
}

impl TryFrom<&ConfigCreateArgs> for Rollup {
    type Error = eyre::Report;

    fn try_from(args: &ConfigCreateArgs) -> eyre::Result<Self> {
        let globals_config = GlobalsConfig::from(args);
        let deployment_config = RollupDeploymentConfig::try_from(args)?;
        let ingress_config = IngressConfig::from(args);
        let celestia_node = CelestiaNode::from(args);

        Ok(Self {
            globals_config,
            deployment_config,
            ingress_config,
            celestia_node,
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
}

/// Describes the ingress config for the rollup chart.
///
/// Serializes to a yaml file for usage with Helm, thus the
/// `rename_all = "camelCase"` naming convention.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngressConfig {
    hostname: String,
}

impl From<&ConfigCreateArgs> for IngressConfig {
    fn from(args: &ConfigCreateArgs) -> Self {
        Self {
            hostname: args.hostname.clone(),
        }
    }
}

/// Describes the globals used for rollup chart.
///
/// Serializes to a yaml file for usage with Helm, thus the
/// `rename_all = "camelCase"` naming convention.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalsConfig {
    pub(crate) namespace: String,
    #[serde(rename = "useTTY")]
    use_tty: bool,
    log_level: String,
}

impl From<&ConfigCreateArgs> for GlobalsConfig {
    fn from(args: &ConfigCreateArgs) -> Self {
        Self {
            namespace: args.namespace.clone(),
            use_tty: args.use_tty,
            log_level: args.log_level.clone(),
        }
    }
}

/// Describes the values for Celestia Node helm chart, which is a dependency
/// of the rollup chart.
///
/// Serializes to a yaml file for usage with Helm, thus the
/// `rename_all = "camelCase"` naming convention.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CelestiaNode {
    enabled: bool,
    #[serde(rename = "config")]
    celestia_node_config: CelestiaNodeConfig,
}

impl From<&ConfigCreateArgs> for CelestiaNode {
    fn from(args: &ConfigCreateArgs) -> Self {
        let celestia_node_config = CelestiaNodeConfig::from(args);

        Self {
            enabled: args.enable_celestia_node,
            celestia_node_config,
        }
    }
}

/// Describes the configuration for a Celestia Node values.
///
/// Serializes to a yaml file for usage with Helm, thus the
/// `rename_all = "camelCase"` naming convention.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CelestiaNodeConfig {
    label_prefix: String,
}

impl From<&ConfigCreateArgs> for CelestiaNodeConfig {
    fn from(args: &ConfigCreateArgs) -> Self {
        Self {
            label_prefix: args.name.to_string(),
        }
    }
}

impl TryFrom<&ConfigCreateArgs> for RollupDeploymentConfig {
    type Error = eyre::Report;

    fn try_from(args: &ConfigCreateArgs) -> eyre::Result<Self> {
        // Set to block 2 if nothing set.
        let sequencer_initial_block_height = args.sequencer_initial_block_height.unwrap_or(2);

        let genesis_accounts = args
            .genesis_accounts
            .clone()
            .into_iter()
            .map(GenesisAccount::from)
            .collect();

        Ok(Self {
            rollup: RollupConfig {
                name: args.name.clone(),
                network_id: args.network_id.to_string(),
                execution_commit_level: args.execution_commit_level.to_string(),
                genesis: GenesisConfig {
                    override_genesis_extra_data: args.override_genesis_extra_data,
                    bridge_address: args.bridge_address.clone(),
                    bridge_allowed_asset_denom: args.bridge_allowed_asset_denom.clone(),
                    alloc: genesis_accounts,
                },
            },
            sequencer: SequencerConfig {
                initial_block_height: sequencer_initial_block_height.to_string(),
                grpc: args.sequencer_grpc.clone(),
                rpc: args.sequencer_rpc.clone(),
                chain_id: args.sequencer_chain_id.clone(),
            },
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollupConfig {
    name: String,
    // NOTE - String here because yaml will serialize large ints w/ scientific notation
    network_id: String,
    execution_commit_level: String,
    genesis: GenesisConfig,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenesisConfig {
    override_genesis_extra_data: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    bridge_address: Option<String>,
    bridge_allowed_asset_denom: String,
    alloc: Vec<GenesisAccount>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenesisAccount {
    address: String,
    value: GenesisAccountValue,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenesisAccountValue {
    // NOTE - string because yaml will serialize large ints w/ scientific notation
    balance: String,
}

impl From<GenesisAccountArg> for GenesisAccount {
    fn from(arg: GenesisAccountArg) -> Self {
        Self {
            address: arg.address,
            value: GenesisAccountValue {
                balance: arg.balance.to_string(),
            },
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SequencerConfig {
    // NOTE - string because yaml will serialize large ints w/ scientific notation
    initial_block_height: String,
    rpc: String,
    grpc: String,
    chain_id: String,
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
            network_id: 1,
            execution_commit_level: "SoftOnly".to_string(),
            override_genesis_extra_data: false,
            bridge_address: None,
            bridge_allowed_asset_denom: "nria".to_string(),
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
            sequencer_initial_block_height: Some(127_689_000_000),
            sequencer_grpc: "http://localhost:8080".to_string(),
            sequencer_rpc: "http://localhost:8081".to_string(),
            sequencer_chain_id: "test-chain-1".to_string(),
            hostname: "test.com".to_string(),
            namespace: "test-cluster".to_string(),
            enable_celestia_node: false,
        };

        let expected_config = Rollup {
            globals_config: GlobalsConfig {
                use_tty: true,
                namespace: "test-cluster".to_string(),
                log_level: "debug".to_string(),
            },
            deployment_config: RollupDeploymentConfig {
                rollup: RollupConfig {
                    name: "rollup1".to_string(),
                    network_id: "1".to_string(),
                    execution_commit_level: "SoftOnly".to_string(),
                    genesis: GenesisConfig {
                        override_genesis_extra_data: false,
                        bridge_address: None,
                        bridge_allowed_asset_denom: "nria".to_string(),
                        alloc: vec![
                            GenesisAccount {
                                address: "0xA5TR14".to_string(),
                                value: GenesisAccountValue {
                                    balance: "1000000000000000000".to_string(),
                                },
                            },
                            GenesisAccount {
                                address: "0x420XYZ69".to_string(),
                                value: GenesisAccountValue {
                                    balance: "420".to_string(),
                                },
                            },
                        ],
                    },
                },
                sequencer: SequencerConfig {
                    initial_block_height: "127689000000".to_string(),
                    grpc: "http://localhost:8080".to_string(),
                    rpc: "http://localhost:8081".to_string(),
                    chain_id: "test-chain-1".to_string(),
                },
            },
            ingress_config: IngressConfig {
                hostname: "test.com".to_string(),
            },
            celestia_node: CelestiaNode {
                enabled: false,
                celestia_node_config: CelestiaNodeConfig {
                    label_prefix: "rollup1".to_string(),
                },
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
            network_id: 2_211_011_801,
            execution_commit_level: "SoftOnly".to_string(),
            override_genesis_extra_data: false,
            bridge_address: None,
            bridge_allowed_asset_denom: "nria".to_string(),
            genesis_accounts: vec![GenesisAccountArg {
                address: "0xA5TR14".to_string(),
                balance: 10000,
            }],
            sequencer_initial_block_height: None,
            sequencer_grpc: "http://localhost:8082".to_string(),
            sequencer_rpc: "http://localhost:8083".to_string(),
            sequencer_chain_id: "test-chain-1".to_string(),
            hostname: "localdev.me".to_string(),
            namespace: "astria-dev-cluster".to_string(),
            enable_celestia_node: false,
        };

        let expected_config = Rollup {
            globals_config: GlobalsConfig {
                use_tty: false,
                namespace: "astria-dev-cluster".to_string(),
                log_level: "info".to_string(),
            },
            deployment_config: RollupDeploymentConfig {
                rollup: RollupConfig {
                    name: "rollup2".to_string(),
                    network_id: "2211011801".to_string(),
                    execution_commit_level: "SoftOnly".to_string(),
                    genesis: GenesisConfig {
                        override_genesis_extra_data: false,
                        bridge_address: None,
                        bridge_allowed_asset_denom: "nria".to_string(),
                        alloc: vec![GenesisAccount {
                            address: "0xA5TR14".to_string(),
                            value: GenesisAccountValue {
                                balance: "10000".to_string(),
                            },
                        }],
                    },
                },
                sequencer: SequencerConfig {
                    initial_block_height: "2".to_string(), // Default value
                    grpc: "http://localhost:8082".to_string(),
                    rpc: "http://localhost:8083".to_string(),
                    chain_id: "test-chain-1".to_string(),
                },
            },
            ingress_config: IngressConfig {
                hostname: "localdev.me".to_string(),
            },
            celestia_node: CelestiaNode {
                enabled: false,
                celestia_node_config: CelestiaNodeConfig {
                    label_prefix: "rollup2".to_string(),
                },
            },
        };

        let result = Rollup::try_from(&args)?;
        assert_eq!(result, expected_config);

        Ok(())
    }
}
