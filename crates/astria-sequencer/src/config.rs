use std::path::PathBuf;

use astria_config_derive::astria_config;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize)]
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
    use astria_utils::{
        config_test_suite_failing,
        config_test_suite_passing,
    };

    use crate::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");
    const ENV_PREFIX: &str = "ASTRIA_SEQUENCER_";

    #[test]
    fn test_config_passing() {
        config_test_suite_passing::<Config>(ENV_PREFIX, EXAMPLE_ENV);
    }

    #[test]
    #[should_panic]
    fn test_config_failing() {
        config_test_suite_failing::<Config>(ENV_PREFIX, EXAMPLE_ENV);
    }
}
