use clap::Parser;
use serde::Serialize;

#[derive(Debug, Parser, Serialize)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// Sequencer node RPC endpoint.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) sequencer_endpoint: Option<String>,
}
