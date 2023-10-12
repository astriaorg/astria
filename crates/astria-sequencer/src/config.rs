use std::path::PathBuf;

use config::astria_config;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug,Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[astria_config(ASTRIA_SEQUENCER_)]
pub struct Config {
    /// The endpoint on which Sequencer will listen for ABCI requests
    pub listen_addr: String,
    /// The path to penumbra storage db.
    pub db_filepath: PathBuf,
    /// Log level: debug, info, warn, or error
    pub log: String,
}

#[cfg(test)]
mod test {
    use crate::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn example_env_config_is_up_to_date() {
        config::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[test]
    #[should_panic]
    fn config_should_reject_unknown_var() {
        config::config_should_reject_unknown_var::<Config>(EXAMPLE_ENV);
    }
}
