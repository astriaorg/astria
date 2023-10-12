use config::astria_config;
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
    pub initial_sequencer_block_height: u32,
}
#[cfg(test)]
mod test {
    use crate::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn example_env_config_is_up_to_date() {
        config::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[test]
    #[should_panic]
    fn config_should_reject_unknown_var() {
        config::config_should_reject_unknown_var::<Config>(EXAMPLE_ENV);
    }
}
