use clap::Parser;
use serde::Serialize;

#[derive(Debug, Parser, Serialize)]
pub struct Cli {
    /// URL of the data layer server.
    #[arg(long = "celestia-node-url")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub celestia_node_url: Option<String>,

    #[arg(long = "tendermint-url")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub tendermint_url: Option<String>,

    /// Chain ID as a string; this should correspond to the `secondaryChainID`
    /// used when transactions are submitted to the sequencer.
    #[arg(long = "chain-id")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub chain_id: Option<String>,

    /// Address of the execution RPC server.
    #[arg(long = "execution-rpc-url")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub execution_rpc_url: Option<String>,

    /// Log level. One of debug, info, warn, or error
    #[arg(long = "log", default_value = "info")]
    pub log_level: String,
}
