use clap::Parser;
use serde::Serialize;

#[derive(Debug, Parser, Serialize)]
pub(crate) struct Cli {
    /// URL of the data layer server.
    #[arg(long = "celestia-node-url")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) celestia_node_url: Option<String>,

    /// Chain ID as a string
    #[arg(long = "chain-id")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) chain_id: Option<String>,

    /// Address of the execution RPC server.
    #[arg(long = "rpc-address")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) rpc_address: Option<String>,
}
