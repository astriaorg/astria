use std::{
    net::{
        AddrParseError,
        SocketAddr,
    },
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

const DEFAULT_SEQUENCER_URL: &str = "127.0.0.1:1317";
const DEFAULT_API_URL: &str = "127.0.0.1:8080";
const DEFAULT_CHAIN_ID: &str = "ethereum";
const DEFAULT_EXECUTION_RPC_URL: &str = "127.0.0.1:50051";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid config")]
    ConfigExtraction(#[from] figment::Error),
    #[error("invalid api_url")]
    InvalidApiUrl(#[from] AddrParseError),
    #[error("missing cli field")]
    MissingCliField(String),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Config {
    /// Address of the API server
    #[serde(default = "default_api_url")]
    pub api_url: SocketAddr,

    /// Address of the RPC server for the sequencer chain
    #[serde(default = "default_sequencer_url")]
    pub sequencer_url: SocketAddr,

    /// Chain ID that we want to connect to
    #[serde(default = "default_chain_id")]
    pub chain_id: ChainId,

    /// Address of the RPC server for execution
    #[serde(default = "default_execution_rpc_url")]
    pub execution_rpc_url: SocketAddr,
}

impl Config {
    /// Constructs [`Config`] with command line arguments.
    ///
    /// The command line arguments have to be explicitly passed in to make
    /// the config logic testable. [`Config::with_cli`] is kept private because
    /// the `[config::get]` utility function is the main entry point
    pub(super) fn with_cli(cli_config: cli::Args) -> Result<Self, figment::Error> {
        // rename searcher ali args from searcher_* to *
        #[derive(Debug, Deserialize, Serialize, PartialEq)]
        struct SearcherArgs {
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            api_url: Option<String>,
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            sequencer_url: Option<String>,
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            chain_id: Option<String>,
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            execution_rpc_url: Option<String>,
        }
        let searcher_args = SearcherArgs {
            api_url: cli_config.searcher_api_url,
            sequencer_url: cli_config.sequencer_url,
            chain_id: cli_config.searcher_chain_id,
            execution_rpc_url: cli_config.searcher_execution_rpc_url,
        };

        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Env::prefixed("ASTRIA_COMPOSER_")) // non-searcher specific env vars, e.g. sequencer_url
            .merge(Env::prefixed("ASTRIA_COMPOSER_SEARCHER_")) // searcher-specific env vars
            .merge(Serialized::defaults(searcher_args))
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

pub(super) fn default_api_url() -> SocketAddr {
    DEFAULT_API_URL.parse().unwrap()
}

pub(super) fn default_sequencer_url() -> SocketAddr {
    DEFAULT_SEQUENCER_URL.parse().unwrap()
}

pub(super) fn default_chain_id() -> ChainId {
    ChainId::from_str(DEFAULT_CHAIN_ID).unwrap()
}

pub(super) fn default_execution_rpc_url() -> SocketAddr {
    DEFAULT_EXECUTION_RPC_URL.parse().unwrap()
}
