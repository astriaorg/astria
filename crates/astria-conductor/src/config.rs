use serde::{
    Deserialize,
    Serialize,
};

/// The global configuration for the driver and its components.
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// URL of the Celestia Node
    #[serde(default = "default_celestia_node_url")]
    pub celestia_node_url: String,

    /// The JWT bearer token supplied with each jsonrpc call
    #[serde(default)]
    pub celestia_bearer_token: String,

    /// URL of the Tendermint node (sequencer/metro)
    #[serde(default = "default_tendermint_url")]
    pub tendermint_url: String,

    /// Chain ID that we want to work in
    #[serde(default = "default_chain_id")]
    pub chain_id: String,

    /// Address of the RPC server for execution
    #[serde(default = "default_execution_rpc_url")]
    pub execution_rpc_url: String,

    /// Disable reading from the DA layer and block finalization
    #[serde(default)]
    pub disable_finalization: bool,

    /// Bootnodes for the P2P network
    #[serde(deserialize_with = "bootnodes_deserialize")]
    pub bootnodes: Vec<String>,

    /// Path to the libp2p private key file
    pub libp2p_private_key: Option<String>,
}

fn bootnodes_deserialize<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(String::deserialize(deserializer)
        .wrap_err("failed to deserialize bootnode string")?
        .split(',')
        .map(|item| item.to_owned())
        .collect())
}

// NOTE - using default fns instead of defaults in Cli because defaults
//   in Cli always override values from a Config file, which we don't want.

fn default_celestia_node_url() -> String {
    "http://localhost:26658".to_string()
}

fn default_tendermint_url() -> String {
    "http://localhost:26657".to_string()
}

fn default_chain_id() -> String {
    "ethereum".to_string()
}

fn default_execution_rpc_url() -> String {
    "http://localhost:50051".to_string()
}
