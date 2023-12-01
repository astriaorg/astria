use std::path::PathBuf;

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The endpoint on which Sequencer will listen for ABCI requests
    pub listen_addr: String,
    /// The path to penumbra storage db.
    pub db_filepath: PathBuf,
    /// Log level: debug, info, warn, or error
    pub log: String,
    /// Set to true to enable the mint component
    /// Only used if the "mint" feature is enabled
    pub enable_mint: bool,
    /// The gRPC endpoint
    pub grpc_addr: String,
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_SEQUENCER_";
}

#[cfg(test)]
mod tests {
    use super::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn example_env_config_is_up_to_date() {
        config::tests::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[test]
    fn config_should_reject_unknown_var() {
        config::tests::config_should_reject_unknown_var::<Config>(EXAMPLE_ENV);
    }
}
