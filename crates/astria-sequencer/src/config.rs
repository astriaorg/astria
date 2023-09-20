use std::path::PathBuf;

use figment::{
    providers::Env,
    Figment,
};
use serde::{
    Deserialize,
    Serialize,
};

// returns an error generated by figment highlighting what went wrong in trying to read the config
pub fn get() -> Result<Config, figment::Error> {
    Config::from_environment()
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The endpoint on which Sequencer will listen for ABCI requests
    pub listen_addr: String,
    /// The path to penumbra storage db.
    pub data_dir: PathBuf,
    /// Log level: debug, info, warn, or error
    pub log: String,
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
