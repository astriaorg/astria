use figment::{
    providers::Env,
    Figment,
};
use serde::{
    Deserialize,
    Deserializer,
    Serialize,
};

const DEFAULT_CELESTIA_NODE_URL: &str = "http://localhost:26658";
const DEFAULT_TENDERMINT_URL: &str = "http://localhost:26657";
const DEFAULT_CHAIN_ID: &str = "ethereum";
const DEFAULT_EXECUTION_RPC_URL: &str = "http://localhost:50051";
const DEFAULT_LOG_LEVEL: &str = "info";
const DEFAULT_LIBP2P_PORT: u16 = 2451;

pub fn get() -> Result<Config, figment::Error> {
    Config::from_environment()
}

/// The global configuration for the driver and its components.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// URL of the Celestia Node
    #[serde(default = "default_celestia_node_url")]
    pub celestia_node_url: String,

    /// The JWT bearer token supplied with each jsonrpc call
    #[serde(default)]
    pub celestia_bearer_token: String,

    /// URL of the Tendermint node (sequencer/metro)
    #[serde(default = "default_tendermint_url")]
    pub tendermint_url: String,

    /// Chain ID that we want to work in
    #[serde(default = "default_chain_id")]
    pub chain_id: String,

    /// Address of the RPC server for execution
    #[serde(default = "default_execution_rpc_url")]
    pub execution_rpc_url: String,

    /// Disable reading from the DA layer and block finalization
    #[serde(default)]
    pub disable_finalization: bool,

    /// Bootnodes for the P2P network
    #[serde(deserialize_with = "bootnodes_deserialize")]
    #[serde(default)]
    pub bootnodes: Option<Vec<String>>,

    /// Path to the libp2p private key file
    #[serde(default)]
    pub libp2p_private_key: Option<String>,

    /// Port to listen on for libp2p
    #[serde(default = "default_libp2p_port")]
    pub libp2p_port: u16,

    /// log directive to use for telemetry.
    #[serde(default = "default_log_level")]
    pub log: String,
}

fn bootnodes_deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let maybe_bootnodes: Option<String> = Option::deserialize(deserializer)?;
    if maybe_bootnodes.is_none() {
        return Ok(None);
    }
    Ok(Some(
        maybe_bootnodes
            .unwrap()
            .split(',')
            .map(|item| item.to_owned())
            .collect(),
    ))
}

fn default_celestia_node_url() -> String {
    DEFAULT_CELESTIA_NODE_URL.to_string()
}

fn default_tendermint_url() -> String {
    DEFAULT_TENDERMINT_URL.to_string()
}

fn default_chain_id() -> String {
    DEFAULT_CHAIN_ID.to_string()
}

fn default_execution_rpc_url() -> String {
    DEFAULT_EXECUTION_RPC_URL.to_string()
}

fn default_libp2p_port() -> u16 {
    DEFAULT_LIBP2P_PORT
}

fn default_log_level() -> String {
    DEFAULT_LOG_LEVEL.to_string()
}

impl Config {
    fn from_environment() -> Result<Config, figment::Error> {
        Figment::new()
            .merge(Env::prefixed("RUST_").split("_").only(&["log"]))
            .merge(Env::prefixed("ASTRIA_CONDUCTOR_"))
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
                jail.set_env(dbg!(key), dbg!(val));
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
            jail.set_env("ASTRIA_CONDUCTOR_FOOBAR", "BAZ");
            Config::from_environment().unwrap();
            Ok(())
        });
    }
}
