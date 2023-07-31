use figment::{
    providers::{
        Env,
        Serialized,
    },
    Figment,
};
use serde::{
    Deserialize,
    Serialize,
};

pub mod constants;

// TODO: add more default values
// potentially move to a separate module so it can be imported into searcher and block_builder?
const DEFAULT_LOG: &str = "info";

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
    Config::new()
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
/// The high-level config for creating an astria-composer service.
pub struct Config {
    /// Log level. One of debug, info, warn, or error
    #[serde(default = "default_log")]
    pub log: String,

    /// Address of the API server
    #[serde(default = "default_api_port")]
    pub api_port: u16,

    /// Address of the RPC server for the sequencer chain
    #[serde(default = "default_sequencer_url")]
    pub sequencer_url: String,

    /// Sequencer address for the bundle signer
    #[serde(default = "default_sequencer_address")]
    pub sequencer_address: String,

    /// Sequencer secret for transaction signing
    #[serde(default = "default_sequencer_address")]
    pub sequencer_secret: String,

    /// Chain ID that we want to connect to
    #[serde(default = "default_chain_id")]
    pub chain_id: String,

    /// Address of the RPC server for execution
    #[serde(default = "default_execution_ws_url")]
    pub execution_ws_url: String,
}

impl Config {
    /// Constructs [`Config`] with command line arguments.
    ///
    /// The command line arguments have to be explicitly passed in to make
    /// the config logic testable. [`Config::with_cli`] is kept private because
    /// the `[config::get]` utility function is the main entry point
    fn new() -> Result<Config, figment::Error> {
        let rust_log = Env::prefixed("RUST_").split("_").only(&["log"]);

        Figment::from(Serialized::defaults(Config::default()))
            .merge(rust_log)
            .merge(Env::prefixed("ASTRIA_COMPOSER_"))
            .extract()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log: default_log(),
            api_port: default_api_port(),
            sequencer_url: default_sequencer_url(),
            sequencer_address: default_sequencer_address(),
            sequencer_secret: default_sequencer_secret(),
            chain_id: default_chain_id(),
            execution_ws_url: default_execution_ws_url(),
        }
    }
}

fn default_log() -> String {
    DEFAULT_LOG.into()
}

fn default_api_port() -> u16 {
    constants::DEFAULT_API_PORT
}

fn default_sequencer_url() -> String {
    constants::DEFAULT_SEQUENCER_URL.to_string()
}

fn default_sequencer_address() -> String {
    constants::DEFAULT_SEQUENCER_ADDRESS.to_string()
}

fn default_sequencer_secret() -> String {
    constants::DEFAULT_SEQUENCER_SECRET.to_string()
}

fn default_chain_id() -> String {
    constants::DEFAULT_CHAIN_ID.to_string()
}

fn default_execution_ws_url() -> String {
    constants::DEFAULT_EXECUTION_WS_URL.to_string()
}

#[cfg(test)]
mod tests {

    use figment::Jail;

    use super::Config;

    fn set_all_env(jail: &mut Jail) {
        jail.set_env("ASTRIA_COMPOSER_LOG", "env=warn");
        jail.set_env("ASTRIA_COMPOSER_API_PORT", "5050");
        jail.set_env("ASTRIA_COMPOSER_SEQUENCER_URL", "127.0.0.1:1210");
        jail.set_env("ASTRIA_COMPOSER_SEQUENCER_SECRET", "envsecret");
        jail.set_env("ASTRIA_COMPOSER_SEQUENCER_ADDRESS", "envaddress");
        jail.set_env("ASTRIA_COMPOSER_CHAIN_ID", "envnet");
        jail.set_env(
            "ASTRIA_COMPOSER_EXECUTION_WS_URL",
            "127.0.0.1:40041",
        );
    }

    #[test]
    fn env_overrides_default() {
        Jail::expect_with(|jail| {
            set_all_env(jail);
            let actual = Config::new().unwrap();
            let expected = Config {
                log: "env=warn".into(),
                sequencer_url: "127.0.0.1:1210".parse().unwrap(),
                sequencer_address: "envaddress".to_string(),
                sequencer_secret: "envsecret".to_string(),
                api_port: 5050,
                chain_id: "envnet".to_string(),
                execution_ws_url: "127.0.0.1:40041".parse().unwrap(),
            };
            assert_eq!(expected, actual);
            Ok(())
        });
    }
}
