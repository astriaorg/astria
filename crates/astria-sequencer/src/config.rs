use std::path::PathBuf;

use astria_config::astria_config;
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
    use astria_config::{
        config_should_reject_unknown_var,
        example_env_config_is_up_to_date,
    };

    use crate::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn test_config_passing() {
        example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[test]
    #[should_panic]
    fn test_config_failing() {
        config_should_reject_unknown_var::<Config>(EXAMPLE_ENV);
    }
}
