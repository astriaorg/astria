use clap::Parser;
use serde::Serialize;

#[derive(Debug, Parser, Serialize, Clone)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// Sequencer node RPC endpoint.
    #[arg(long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) sequencer_url: Option<String>,

    /// Address of the API server
    #[arg(long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) searcher_api_port: Option<u16>,

    /// Chain ID that we want to connect to
    #[arg(long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) searcher_chain_id: Option<String>,

    /// Address of the RPC server for execution
    #[arg(long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) searcher_execution_ws_url: Option<String>,
}
