use clap::Parser;

#[derive(Debug, Parser)]
pub struct Config {
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
