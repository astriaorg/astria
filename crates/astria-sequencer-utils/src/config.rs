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
    #[clap(long, short = 's')]
    pub source_genesis_file: String,

    #[clap(long, short = 'd')]
    pub destination_genesis_file: String,
}

impl Config {
    #[must_use]
    pub fn get() -> Self {
        Config::parse()
    }
}
