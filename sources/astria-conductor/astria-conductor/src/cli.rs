use clap::Parser;
use serde::Serialize;

#[derive(Debug, Parser, Serialize)]
pub(crate) struct Cli {
    /// URL of the data layer server.
    #[arg(long = "celestia-node-url")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) celestia_node_url: Option<String>,

    /// Namespace ID as a string; the hex encoding of a [u8; 8]
    #[arg(long = "namespace-id")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) namespace_id: Option<String>,

    /// Address of the execution RPC server.
    #[arg(long = "rpc-address")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) rpc_address: Option<String>,
}
