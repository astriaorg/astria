use std::{
    net::AddrParseError,
    str::FromStr,
};

use figment::{
    providers::{
        Env,
        Serialized,
    },
    Figment,
};
use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;

use super::cli;
use crate::types::rollup::ChainId;

const DEFAULT_SEQUENCER_URL: &str = "http://localhost:1317";
const DEFAULT_API_URL: &str = "http://localhost:808080";
const DEFAULT_CHAIN_ID: &str = "ethereum";
const DEFAULT_EXECUTION_RPC_URL: &str = "http://localhost:50051";

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
    #[serde(default = "default_sequencer_url")]
    pub sequencer_url: String,

    /// Address of the API server
    #[serde(default = "default_api_url")]
    pub api_url: String,

    /// Chain ID that we want to connect to
    #[serde(default = "default_chain_id")]
    pub chain_id: ChainId,

    /// Address of the RPC server for execution
    #[serde(default = "default_execution_rpc_url")]
    pub execution_rpc_url: String,
}

impl Config {
    /// Constructs [`Config`] with command line arguments.
    ///
    /// The command line arguments have to be explicitly passed in to make
    /// the config logic testable. [`Config::with_cli`] is kept private because
    /// the `[config::get]` utility function is the main entry point
    pub(super) fn with_cli(cli_config: cli::Args) -> Result<Self, figment::Error> {
        // grab the cli args that we want to pass to the searcher
        #[derive(Serialize)]
        struct SearcherCliArgs {
            sequencer_url: Option<String>,
            api_url: Option<String>,
            chain_id: Option<String>,
            execution_rpc_url: Option<String>,
        }
        let searcher_cli_args = SearcherCliArgs {
            sequencer_url: cli_config.sequencer_url,
            api_url: cli_config.searcher_api_url,
            chain_id: cli_config.searcher_chain_id,
            execution_rpc_url: cli_config.searcher_execution_rpc_url,
        };

        // merge the cli args with the defaults and env
        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Env::prefixed("ASTRIA_COMPOSER_")) // non-searcher specific env vars, e.g. sequencer_url
            .merge(Env::prefixed("ASTRIA_COMPOSER_SEARCHER_")) // searcher-specific env vars
            .merge(Serialized::defaults(searcher_cli_args))
            .extract()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sequencer_url: default_sequencer_url(),
            chain_id: default_chain_id(),
            execution_rpc_url: default_execution_rpc_url(),
            api_url: default_api_url(),
        }
    }
}

pub(super) fn default_sequencer_url() -> String {
    DEFAULT_SEQUENCER_URL.to_string()
}

pub(super) fn default_chain_id() -> ChainId {
    ChainId::from_str(DEFAULT_CHAIN_ID).unwrap()
}

pub(super) fn default_execution_rpc_url() -> String {
    DEFAULT_EXECUTION_RPC_URL.to_string()
}

pub(super) fn default_api_url() -> String {
    DEFAULT_API_URL.to_string()
}
