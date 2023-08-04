use std::net::SocketAddr;

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
/// This includes parsing the command line args, getting all environment variables.
/// This function will short circuit binary execution when `--help` or `--version`
/// is provided, or if the command line arguments could not be read.
///
/// # Errors
///
/// An error is returned if the config could not be read.
pub fn get() -> Result<Config, figment::Error> {
    Config::from_environment()
}

/// The high-level config for creating an astria-composer service.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Log level. One of debug, info, warn, or error
    pub log: String,

    /// Address of the API server
    pub api_listen_addr: SocketAddr,

    /// Address of the RPC server for the sequencer chain
    pub sequencer_url: String,

    /// Chain ID that we want to connect to
    pub chain_id: String,

    /// Address of the RPC server for execution
    pub execution_url: String,
}

impl Config {
    /// Constructs [`Config`] with command line arguments.
    ///
    /// The command line arguments have to be explicitly passed in to make
    /// the config logic testable. [`Config::with_cli`] is kept private because
    /// the `[config::get]` utility function is the main entry point
    fn from_environment() -> Result<Config, figment::Error> {
        let rust_log = Env::prefixed("RUST_").split("_").only(&["log"]);

        Figment::new()
            .merge(rust_log)
            .merge(Env::prefixed("ASTRIA_COMPOSER_"))
            .extract()
    }
}

#[cfg(test)]
mod tests {
    use figment::Jail;

    use super::Config;
    const EXAMPLE_ENV: &str = include_str!("../example.env");

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
            jail.set_env("ASTRIA_COMPOSER_FOOBAR", "BAZ");
            Config::from_environment().unwrap();
            Ok(())
        });
    }
}
