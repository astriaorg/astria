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

    /// Comma-separated string of libp2p addresses of nodes to connect to.
    #[arg(long = "bootnodes")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub bootnodes: Option<String>,

    /// Path to the libp2p private key file.
    #[arg(long = "libp2p-private-key")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub libp2p_private_key: Option<String>,

    #[arg(long = "libp2p-port")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub libp2p_port: Option<u16>,

    #[arg(long = "disable-finalization")]
    pub disable_finalization: bool,
}
