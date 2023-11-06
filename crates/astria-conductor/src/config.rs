//! The conductor configuration.

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CommitLevel {
    SoftOnly,
    FirmOnly,
    SoftAndFirm,
}

impl CommitLevel {
    pub fn is_soft_only(&self) -> bool {
        matches!(self, Self::SoftOnly)
    }

    pub fn is_firm_only(&self) -> bool {
        matches!(self, Self::FirmOnly)
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

    /// log directive to use for telemetry.
    pub log: String,

    /// Choose to execute empty blocks or not
    pub disable_empty_block_execution: bool,

    /// The Sequencer block height that the rollup genesis block was in
    pub initial_sequencer_block_height: u32,

    /// The DA block height that the rollup genesis block was in
    pub initial_da_block_height: u32,

    /// The execution commit level used for controlling how blocks are sent to
    /// the execution layer.
    pub execution_commit_level: CommitLevel,
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_CONDUCTOR_";
}

#[cfg(test)]
mod tests {
    use super::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn example_env_config_is_up_to_date() {
        config::tests::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[test]
    fn config_should_reject_unknown_var() {
        config::tests::config_should_reject_unknown_var::<Config>(EXAMPLE_ENV);
    }
}
