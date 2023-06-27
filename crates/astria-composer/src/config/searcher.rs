use std::{
    net::AddrParseError,
    str::FromStr,
};

use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;

use crate::types::rollup::ChainId;

const DEFAULT_SEQUENCER_ENDPOINT: &str = "http://localhost:1317";
const DEFAULT_CHAIN_ID: &str = "ethereum";
const DEFAULT_EXECUTION_RPC_URL: &str = "http://localhost:50051";
const DEFAULT_API_URL: &str = "http://localhost:808080";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid config")]
    InvalidConfig(),
    #[error("invalid api_url")]
    InvalidApiUrl(#[from] AddrParseError),
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Config {
    /// Address of the RPC server for the sequencer chain
    #[serde(default = "default_sequencer_endpoint")]
    pub sequencer_endpoint: String,

    /// Chain ID that we want to work in
    #[serde(default = "default_chain_id")]
    pub chain_id: ChainId,

    /// Address of the RPC server for execution
    #[serde(default = "default_execution_rpc_url")]
    pub execution_rpc_url: String,

    /// Address of the RPC server for execution
    #[serde(default = "default_api_url")]
    pub api_url: String,
}

impl Default for Config {
    fn default() -> Self {
        unimplemented!()
    }
}

fn default_sequencer_endpoint() -> String {
    DEFAULT_SEQUENCER_ENDPOINT.to_string()
}

fn default_chain_id() -> ChainId {
    ChainId::from_str(DEFAULT_CHAIN_ID).unwrap()
}

fn default_execution_rpc_url() -> String {
    DEFAULT_EXECUTION_RPC_URL.to_string()
}

fn default_api_url() -> String {
    DEFAULT_API_URL.to_string()
}
