use std::path::PathBuf;

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The endpoint on which Sequencer will listen for ABCI requests
    pub listen_addr: String,
    /// The path to penumbra storage db.
    pub db_filepath: PathBuf,
    /// Log level: debug, info, warn, or error
    pub log: String,
}

impl config::AstriaConfig for Config {
    const PREFIX: &'static str = "ASTRIA_SEQUENCER_";
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
    fn test_config_failing() {
        config::config_should_reject_unknown_var::<Config>(EXAMPLE_ENV);
    }
}
