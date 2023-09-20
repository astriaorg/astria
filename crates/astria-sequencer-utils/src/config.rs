use clap::Parser;

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(long)]
    pub genesis_app_state_file: String,

    #[clap(long)]
    pub destination_genesis_file: String,
}

impl Config {
    #[must_use]
    pub fn get() -> Self {
        Config::parse()
    }
}
