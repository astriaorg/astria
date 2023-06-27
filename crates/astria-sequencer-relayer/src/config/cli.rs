use clap::Parser;
use serde::Serialize;

/// Relays blocks from the astria shared sequencer
/// to a data availability layer (currently celestia).
#[derive(Debug, Parser, Serialize)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// Sequencer node RPC endpoint.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) sequencer_endpoint: Option<String>,

    /// Celestia node RPC endpoint.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) celestia_endpoint: Option<String>,

    /// Gas limit for transactions sent to Celestia.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) gas_limit: Option<u64>,

    /// Disable writing the sequencer block to Celestia.
    #[arg(long)]
    #[serde(skip_serializing_if = "::std::ops::Not::not")]
    pub(crate) disable_writing: bool,

    /// Disable publishing block to the network.
    #[arg(long)]
    #[serde(skip_serializing_if = "::std::ops::Not::not")]
    pub(crate) disable_network: bool,

    /// Expected block time of the sequencer in milliseconds;
    /// ie. how often we should poll the sequencer.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) sequencer_block_time: Option<u64>,

    /// Expected block time of Celestia in milliseconds;
    /// ie. how often we should write to Celestia.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) celestia_block_time: Option<u64>,

    /// Path to validator private key file.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) validator_key_file: Option<String>,

    /// RPC port to listen on.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) rpc_port: Option<u16>,

    /// P2P port to listen on.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) p2p_port: Option<u16>,

    /// Log level. One of debug, info, warn, or error
    #[arg(short, long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub(crate) log: Option<String>,
}
