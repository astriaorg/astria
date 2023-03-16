use serde::{Deserialize, Serialize};

/// The global configuration for the driver and its components.
#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    /// URL of the Celestia Node
    pub(crate) celestia_node_url: String,

    /// Namespace that we want to work in
    pub(crate) namespace_id: String,

    /// Address of the RPC server for execution
    pub(crate) rpc_address: String,
}
