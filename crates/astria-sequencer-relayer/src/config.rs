use figment::{
    providers::Env,
    Figment,
};
use serde::{
    Deserialize,
    Serialize,
};

/// Utility function to read the application's config in one go.
///
/// # Errors
///
/// An error is returned if the config could not be read.
pub fn get() -> Result<Config, figment::Error> {
    Config::from_environment()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
/// The single config for creating an astria-sequencer-relayer service.
#[serde(deny_unknown_fields)]
pub struct Config {
    pub sequencer_endpoint: String,
    pub celestia_endpoint: String,
    pub celestia_bearer_token: String,
    pub gas_limit: u64,
    pub validator_key_file: String,
    pub rpc_port: u16,
    pub log: String,
}

impl Config {
    /// Constructs [`Config`] with command line arguments.
    fn from_environment() -> Result<Config, figment::Error> {
        let rust_log = Env::prefixed("RUST_").split("_").only(&["log"]);

        Figment::new()
            .merge(rust_log)
            .merge(Env::prefixed("ASTRIA_SEQUENCER_RELAYER_"))
            .extract()
    }
}

#[cfg(test)]
mod tests {
    use figment::Jail;
    use once_cell::sync::Lazy;
    use regex::Regex;

    use super::Config;
    const EXAMPLE_ENV: &str =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/local.env.example"));

    fn populate_environment_from_example(jail: &mut Jail) {
        static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[[:space:]]+").unwrap());
        static RE_END: Lazy<Regex> = Lazy::new(|| Regex::new(r"[[:space:]]+$").unwrap());
        for line in EXAMPLE_ENV.lines() {
            if let Some((key, val)) = line.trim().split_once('=') {
                if RE_END.is_match(key) || RE_START.is_match(val) {
                    panic!("env vars must not contain spaces in assignment\n{line}");
                }
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
            jail.set_env("ASTRIA_SEQUENCER_RELAYER_FOOBAR", "BAZ");
            Config::from_environment().unwrap();
            Ok(())
        });
    }
}
