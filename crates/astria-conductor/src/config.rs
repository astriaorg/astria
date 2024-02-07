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
    pub(crate) fn is_soft_only(&self) -> bool {
        matches!(self, Self::SoftOnly)
    }

    pub(crate) fn is_firm_only(&self) -> bool {
        matches!(self, Self::FirmOnly)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// URL of the Celestia Node
    pub celestia_node_url: String,

    /// The JWT bearer token supplied with each jsonrpc call
    pub celestia_bearer_token: String,

    /// URL of the sequencer cometbft websocket
    pub sequencer_url: String,

    /// Address of the RPC server for execution
    pub execution_rpc_url: String,

    /// log directive to use for telemetry.
    pub log: String,

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
}
