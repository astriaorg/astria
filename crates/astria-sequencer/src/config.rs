use clap::Parser;
use serde::{
    Deserialize,
    Serialize,
};

const DEFAULT_ABCI_LISTEN_ADDR: &str = "127.0.0.1:26658";

#[derive(Debug, Deserialize, Parser, Serialize)]
pub struct Config {
    /// The endpoint on which Sequencer will listen for ABCI requests
    #[arg(long, default_value_t = String::from(DEFAULT_ABCI_LISTEN_ADDR))]
    pub(crate) listen_addr: String,
    /// The path to the json encoded genesis file with a list of accounts.
    #[arg(long)]
    pub(crate) genesis_file: String,
}

impl Config {
    pub fn get() -> Self {
        Config::parse()
    }
}
