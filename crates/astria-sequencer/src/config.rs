use std::path::PathBuf;

use figment::{
    providers::Env,
    Figment,
};
use serde::{
    Deserialize,
    Serialize,
};

/// # Errors
///
/// If figment failed to read the config from the environment
pub fn get() -> Result<Config, figment::Error> {
    Config::from_environment("ASTRIA_SEQUENCER_")
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The endpoint on which Sequencer will listen for ABCI requests
    pub listen_addr: String,
    /// The path to penumbra storage db.
    pub db_filepath: PathBuf,
    /// Log level: debug, info, warn, or error
    pub log: String,
}

impl Config {
    fn from_environment(envar_prefix: &str) -> Result<Config, figment::Error> {
        Figment::new()
            .merge(Env::prefixed("RUST_").split("_").only(&["log"]))
            .merge(Env::prefixed(envar_prefix))
            .extract()
    }
}

#[cfg(test)]
mod tests {
    use figment::Jail;
    use once_cell::sync::Lazy;
    use regex::Regex;

    use super::Config;
    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    fn populate_environment_from_example(jail: &mut Jail, test_envar_prefix: &str) {
        static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[[:space:]]+").unwrap());
        static RE_END: Lazy<Regex> = Lazy::new(|| Regex::new(r"[[:space:]]+$").unwrap());
        for line in EXAMPLE_ENV.lines() {
            if let Some((key, val)) = line.trim().split_once('=') {
                assert!(
                    !(RE_END.is_match(key) || RE_START.is_match(val)),
                    "env vars must not contain spaces in assignment\n{line}"
                );
                let prefixed_key = format!("{test_envar_prefix}_{key}");
                jail.set_env(prefixed_key, val);
            }
        }
    }

    #[test]
    fn ensure_example_env_is_in_sync() {
        let test_envar_prefix = "TESTTEST";
        let full_envar_prefix = format!("{}_{}", test_envar_prefix, "ASTRIA_SEQUENCER_");
        Jail::expect_with(|jail| {
            populate_environment_from_example(jail, test_envar_prefix);
            Config::from_environment(full_envar_prefix.as_str()).unwrap();
            Ok(())
        });
    }

    #[test]
    #[should_panic]
    fn extra_env_vars_are_rejected() {
        let test_envar_prefix = "TESTTEST";
        let full_envar_prefix = format!("{}_{}", test_envar_prefix, "ASTRIA_SEQUENCER_");
        Jail::expect_with(|jail| {
            populate_environment_from_example(jail, test_envar_prefix);
            jail.set_env("TESTTEST_ASTRIA_SEQUENCER_FOOBAR", "BAZ");
            Config::from_environment(full_envar_prefix.as_str()).unwrap();
            Ok(())
        });
    }
}
