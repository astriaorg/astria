use clap::Parser;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Deserialize, Parser, Serialize)]
pub struct Config {
    #[clap(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, Deserialize, Parser, Serialize)]
pub enum Command {
    #[clap(name = "genesis-parse")]
    GenesisParser(GenesisParserArgs),
}

#[derive(Debug, Deserialize, Parser, Serialize)]
pub struct GenesisParserArgs {
    #[clap(long)]
    pub sequencer_genesis_file: String,

    #[clap(long)]
    pub cometbft_genesis_file: String,
}

impl Config {
    #[must_use]
    pub fn get() -> Self {
        Config::parse()
    }
}
