use std::path::PathBuf;

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
    pub listen_addr: String,
    /// The path to penumbra storage db.
    #[arg(long)]
    pub db_filepath: PathBuf,
    /// Filter directives for emitting events
    #[arg(long)]
    pub log: Option<String>,
}

impl Config {
    #[must_use]
    pub fn get() -> Self {
        let mut config = Config::parse();
        match std::env::var("RUST_LOG") {
            Ok(log) => {
                if config.log.is_none() {
                    config.log.replace(log);
                }
            }
            Err(err) => {
                eprintln!("ignoring RUST_LOG env var: {err:?}");
            }
        }
        config
    }
}
