use std::path::PathBuf;

use figment::{
    providers::Env,
    Figment,
};
use serde::{
    Deserialize,
    Serialize,
};

const DEFAULT_ABCI_LISTEN_ADDR: &str = "127.0.0.1:26658";
const DEFAULT_LOG: &str = "info";

pub fn get() -> Result<Config, figment::Error> {
    Config::from_environment()
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The endpoint on which Sequencer will listen for ABCI requests
    #[serde(default = "default_abci_addr")]
    pub listen_addr: String,
    /// The path to penumbra storage db.
    pub db_filepath: PathBuf,
    /// Log level: debug, info, warn, or error
    #[serde(default = "default_log")]
    pub log: String,
}

fn default_abci_addr() -> String {
    DEFAULT_ABCI_LISTEN_ADDR.to_string()
}

fn default_log() -> String {
    DEFAULT_LOG.to_string()
}

impl Config {
    fn from_environment() -> Result<Config, figment::Error> {
        Figment::new()
            .merge(Env::prefixed("RUST_").split("_").only(&["log"]))
            .merge(Env::prefixed("ASTRIA_SEQUENCER_"))
            .extract()
    }
}

#[cfg(test)]
mod tests {
    use figment::Jail;

    use super::Config;
    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    fn populate_environment_from_example(jail: &mut Jail) {
        for line in EXAMPLE_ENV.lines() {
            if let Some((key, val)) = line.trim().split_once('=') {
                jail.set_env(key, val);
            }
        }
    }

    #[test]
    fn ensure_example_env_is_in_sync() {
        Jail::expect_with(|jail| {
            populate_environment_from_example(jail);
            Config::from_environment().unwrap();
            Ok(())
        });
    }

    #[test]
    #[should_panic]
    fn extra_env_vars_are_rejected() {
        Jail::expect_with(|jail| {
            populate_environment_from_example(jail);
            jail.set_env("ASTRIA_SEQUENCER_FOOBAR", "BAZ");
            Config::from_environment().unwrap();
            Ok(())
        });
    }
}
