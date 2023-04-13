use serde::{Deserialize, Serialize};

/// The global configuration for the driver and its components.
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// URL of the Celestia Node
    pub celestia_node_url: String,

    /// URL of the Tendermint node (sequencer/metro)
    pub tendermint_url: String,

    /// Chain ID that we want to work in
    pub chain_id: String,

    /// Address of the RPC server for execution
    pub execution_rpc_url: String,
}
