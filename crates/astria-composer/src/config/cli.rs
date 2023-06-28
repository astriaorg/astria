use clap::Parser;
use serde::Serialize;

#[derive(Debug, Parser, Serialize, Clone)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// Log level. One of debug, info, warn, or error
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) log: Option<String>,

    /// Sequencer node RPC endpoint.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) sequencer_url: Option<String>,

    /// Address of the API server
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) searcher_api_url: Option<String>,

    /// Chain ID that we want to connect to
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) searcher_chain_id: Option<String>,

    /// Address of the RPC server for execution
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) searcher_execution_rpc_url: Option<String>,
}
