use std::net::{
    AddrParseError,
    SocketAddr,
};

use astria_sequencer::accounts::types::Address;
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

const DEFAULT_API_PORT: u16 = 8080;
const DEFAULT_SEQUENCER_URL: &str = "sequencer.astria.localdev.me";
const DEFAULT_SEQUENCER_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
const DEFAULT_CHAIN_ID: &str = "912559";
const DEFAULT_EXECUTION_WS_URL: &str = "ws-executor.astria.localdev.me";

#[derive(Debug, Error)]
pub enum Error {
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
    #[serde(default = "default_api_port")]
    pub api_port: u16,

    /// Address of the RPC server for the sequencer chain
    #[serde(default = "default_sequencer_url")]
    pub sequencer_url: String,

    /// Sequencer address for the bundle signer
    pub sequencer_address: Address,

    /// Chain ID that we want to connect to
    #[serde(default = "default_chain_id")]
    pub chain_id: String,

    /// Address of the RPC server for execution
    #[serde(default = "default_execution_ws_url")]
    pub execution_ws_url: String,
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
            api_port: Option<u16>,
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            sequencer_url: Option<String>,
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            chain_id: Option<String>,
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            execution_ws_url: Option<String>,
        }
        let searcher_args = SearcherArgs {
            api_port: cli_config.searcher_api_port,
            sequencer_url: cli_config.sequencer_url,
            chain_id: cli_config.searcher_chain_id,
            execution_ws_url: cli_config.searcher_execution_ws_url,
        };

        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Env::prefixed("ASTRIA_COMPOSER_")) // non-searcher specific env vars, e.g. sequencer_url
            .merge(Env::prefixed("ASTRIA_COMPOSER_SEARCHER_")) // searcher-specific env vars
            .merge(Serialized::defaults(searcher_args))
            .extract()
    }

    /// Returns the API URL from the port specified in config
    ///
    /// # Errors
    /// Wraps the parse error with a [`Error::InvalidApiUrl`]
    pub fn api_url(port: u16) -> Result<SocketAddr, Error> {
        format!("127.0.0.1:{port}")
            .parse()
            .map_err(Error::InvalidApiUrl)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_port: default_api_port(),
            sequencer_url: default_sequencer_url(),
            sequencer_address: default_sequencer_address(),
            chain_id: default_chain_id(),
            execution_ws_url: default_execution_ws_url(),
        }
    }
}

pub(super) fn default_api_port() -> u16 {
    DEFAULT_API_PORT
}

pub(super) fn default_sequencer_url() -> String {
    DEFAULT_SEQUENCER_URL.to_string()
}

pub(super) fn default_sequencer_address() -> Address {
    Address::try_from_str(DEFAULT_SEQUENCER_ADDRESS).unwrap()
}

pub(super) fn default_chain_id() -> String {
    DEFAULT_CHAIN_ID.to_string()
}

pub(super) fn default_execution_ws_url() -> String {
    DEFAULT_EXECUTION_WS_URL.to_string()
}
