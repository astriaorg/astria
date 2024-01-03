//! The conductor configuration.

use std::fmt;

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

impl fmt::Display for CommitLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommitLevel::SoftOnly => write!(f, "SoftOnly"),
            CommitLevel::FirmOnly => write!(f, "FirmOnly"),
            CommitLevel::SoftAndFirm => write!(f, "SoftAndFirm"),
        }
    }
}

impl CommitLevel {
    pub(crate) fn is_soft_only(&self) -> bool {
        matches!(self, Self::SoftOnly)
    }

    pub(crate) fn is_firm_only(&self) -> bool {
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

    /// The Sequencer block height that the rollup genesis block was in
    pub initial_sequencer_block_height: u32,

    /// The DA block height that the rollup's first block was in
    pub initial_celestia_block_height: u32,

    /// The number of block on Celestia in which the first sequencer block for the rollup should be
    /// found
    pub celestia_search_window: u32,

    /// The execution commit level used for controlling how blocks are sent to
    /// the execution layer.
    pub execution_commit_level: CommitLevel,

    /// Set to true to enable OP-Stack deposit derivation.
    pub enable_optimism: bool,

    /// Websocket URL of Ethereum L1 node.
    /// Only used if `enable_optimism` is true.
    pub ethereum_l1_url: String,

    /// Contract address of the OptimismPortal contract on L1.
    /// Only used if `enable_optimism` is true.
    pub optimism_portal_contract_address: String,

    /// The block height of the Ethereum L1 chain that the
    /// OptimismPortal contract was deployed at.
    /// Only used if `enable_optimism` is true.
    pub initial_ethereum_l1_block_height: u64,
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
