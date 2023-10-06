use color_eyre::eyre;
use serde::{
    Deserialize,
    Serialize,
};

use crate::cli::rollup::ConfigCreateArgs;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rollup {
    // TODO - namespace
    #[serde(rename = "config")]
    pub(crate) deployment_config: RollupDeploymentConfig,
}

impl Rollup {
    pub(crate) fn from_cli_args(args: &ConfigCreateArgs) -> eyre::Result<Self> {
        let chain_id = args
            .chain_id
            .clone()
            .unwrap_or(format!("{}-chain", args.name));

        let sequencer_initial_block_height = args.sequencer_initial_block_height.unwrap_or({
            // TODO - get current block height from sequencer
            0
        });

        // FIXME - this should probably be done by implementing traits on GenesisAccountArg
        // let genesis_accounts = args
        //     .genesis_accounts
        //     .iter()
        //     .map(|account| {
        //         let address = account.get_address();
        //         let balance = account.get_balance();
        //         GenesisAccount {
        //             address,
        //             balance,
        //         }
        //     })
        //     .collect();

        let deployment_config = RollupDeploymentConfig {
            use_tty: args.use_tty,
            log_level: args.log_level.clone(),
            rollup: RollupConfig {
                name: args.name.clone(),
                chain_id,
                network_id: args.network_id,
                skip_empty_blocks: args.skip_empty_blocks,
                // genesis_accounts,
            },
            faucet: FaucetConfig {
                private_key: args.faucet_private_key.clone(),
            },
            sequencer: SequencerConfig {
                initial_block_height: sequencer_initial_block_height,
                websocket: args.sequencer_websocket.clone(),
                rpc: args.sequencer_rpc.clone(),
                private_key: args.sequencer_private_key.clone(),
            },
            celestia: CelestiaConfig {
                full_node_url: args.celestia_full_node_url.clone(),
            },
        };

        Ok(Self {
            deployment_config,
        })
    }

    pub(crate) fn to_yaml(&self) -> eyre::Result<String> {
        let yaml_str = serde_yaml::to_string(&self)?;
        Ok(yaml_str)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollupDeploymentConfig {
    use_tty: bool,
    log_level: String,
    rollup: RollupConfig,
    faucet: FaucetConfig,
    sequencer: SequencerConfig,
    celestia: CelestiaConfig,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollupConfig {
    name: String,
    chain_id: String,
    network_id: u64,
    skip_empty_blocks: bool,
    // TODO - best place to ensure this is false?
    // set manually with flag when calling helm install?
    // disable_finalization: bool,
    // genesis_accounts: Vec<GenesisAccount>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenesisAccount {
    address: String,
    balance: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FaucetConfig {
    private_key: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SequencerConfig {
    initial_block_height: u64,
    websocket: String,
    rpc: String,
    private_key: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CelestiaConfig {
    full_node_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_cli_args() -> eyre::Result<()> {
        // Case 1: All args provided
        let args1 = ConfigCreateArgs {
            use_tty: true,
            log_level: "debug".to_string(),
            name: "rollup1".to_string(),
            chain_id: Some("chain1".to_string()),
            network_id: 1,
            skip_empty_blocks: true,
            faucet_private_key: "key1".to_string(),
            sequencer_initial_block_height: Some(10),
            sequencer_websocket: "ws://localhost:8080".to_string(),
            sequencer_rpc: "http://localhost:8081".to_string(),
            sequencer_private_key: "seq_key1".to_string(),
            celestia_full_node_url: "http://celestia_node".to_string(),
        };

        let expected_config1 = Rollup {
            deployment_config: RollupDeploymentConfig {
                use_tty: true,
                log_level: "debug".to_string(),
                rollup: RollupConfig {
                    name: "rollup1".to_string(),
                    chain_id: "chain1".to_string(),
                    network_id: 1,
                    skip_empty_blocks: true,
                },
                faucet: FaucetConfig {
                    private_key: "key1".to_string(),
                },
                sequencer: SequencerConfig {
                    initial_block_height: 10,
                    websocket: "ws://localhost:8080".to_string(),
                    rpc: "http://localhost:8081".to_string(),
                    private_key: "seq_key1".to_string(),
                },
                celestia: CelestiaConfig {
                    full_node_url: "http://celestia_node".to_string(),
                },
            },
        };

        let result1 = Rollup::from_cli_args(&args1)?;
        assert_eq!(result1, expected_config1);

        // Case 2: No optional args provided, should default
        let args2 = ConfigCreateArgs {
            use_tty: false,
            log_level: "info".to_string(),
            name: "rollup2".to_string(),
            chain_id: None,
            network_id: 2,
            skip_empty_blocks: false,
            faucet_private_key: "key2".to_string(),
            sequencer_initial_block_height: None,
            sequencer_websocket: "ws://localhost:8082".to_string(),
            sequencer_rpc: "http://localhost:8083".to_string(),
            sequencer_private_key: "seq_key2".to_string(),
            celestia_full_node_url: "http://celestia_node2".to_string(),
        };

        let expected_config2 = Rollup {
            deployment_config: RollupDeploymentConfig {
                use_tty: false,
                log_level: "info".to_string(),
                rollup: RollupConfig {
                    name: "rollup2".to_string(),
                    chain_id: "rollup2-chain".to_string(), // Derived from name
                    network_id: 2,
                    skip_empty_blocks: false,
                },
                faucet: FaucetConfig {
                    private_key: "key2".to_string(),
                },
                sequencer: SequencerConfig {
                    initial_block_height: 0, // Default value
                    websocket: "ws://localhost:8082".to_string(),
                    rpc: "http://localhost:8083".to_string(),
                    private_key: "seq_key2".to_string(),
                },
                celestia: CelestiaConfig {
                    full_node_url: "http://celestia_node2".to_string(),
                },
            },
        };

        let result2 = Rollup::from_cli_args(&args2)?;
        assert_eq!(result2, expected_config2);

        Ok(())
    }
}
