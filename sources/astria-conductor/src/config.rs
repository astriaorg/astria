use serde::{Deserialize, Serialize};

/// The global configuration for the driver and its components.
#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    /// URL of the Celestia Node
    pub(crate) celestia_node_url: String,

    /// Chain ID that we want to work in
    pub(crate) chain_id: String,

    /// Address of the RPC server for execution
    pub(crate) rpc_address: String,
}
