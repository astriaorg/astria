use astria_config_derive::astria_config;
use serde::{
    Deserialize,
    Serialize,
};

/// The global configuration for the driver and its components.
#[astria_config(ASTRIA_CONDUCTOR_)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// URL of the Celestia Node
    pub celestia_node_url: String,

    /// The JWT bearer token supplied with each jsonrpc call
    pub celestia_bearer_token: String,

    /// URL of the Tendermint node (sequencer/metro)
    pub tendermint_url: String,

    /// URL of the sequencer cometbft websocket
    pub sequencer_url: String,

    /// Chain ID that we want to work in
    pub chain_id: String,

    /// Address of the RPC server for execution
    pub execution_rpc_url: String,

    /// Disable reading from the DA layer and block finalization
    pub disable_finalization: bool,

    /// log directive to use for telemetry.
    pub log: String,

    /// Choose to execute empty blocks or not
    pub disable_empty_block_execution: bool,

    /// The Sequencer block height that the rollup genesis block was in
    pub initial_sequencer_block_height: u64,
}
#[cfg(test)]
mod test {
    use astria_utils::{
        config_test_suite_failing,
        config_test_suite_passing,
    };

    use crate::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");
    const ENV_PREFIX: &str = "ASTRIA_CONDUCTOR_";

    #[test]
    fn test_config_passing() {
        config_test_suite_passing::<Config>(ENV_PREFIX, EXAMPLE_ENV);
    }

    #[test]
    #[should_panic]
    fn test_config_failing() {
        config_test_suite_failing::<Config>(ENV_PREFIX, EXAMPLE_ENV);
    }
}
