use figment::{
    providers::Env,
    Figment,
};
use serde::{
    Deserialize,
    Deserializer,
    Serialize,
};

pub fn get() -> Result<Config, figment::Error> {
    Config::from_environment(None)
}

/// The global configuration for the driver and its components.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// URL of the Celestia Node
    pub celestia_node_url: String,

    /// The JWT bearer token supplied with each jsonrpc call
    pub celestia_bearer_token: String,

    /// URL of the Tendermint node (sequencer/metro)
    pub tendermint_url: String,

    /// Chain ID that we want to work in
    pub chain_id: String,

    /// Address of the RPC server for execution
    pub execution_rpc_url: String,

    /// Disable reading from the DA layer and block finalization
    pub disable_finalization: bool,

    /// Bootnodes for the P2P network
    #[serde(deserialize_with = "bootnodes_deserialize")]
    pub bootnodes: Option<Vec<String>>,

    /// Path to the libp2p private key file
    pub libp2p_private_key: Option<String>,

    /// Port to listen on for libp2p
    pub libp2p_port: u16,

    /// log directive to use for telemetry.
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

impl Config {
    fn from_environment(prefix: Option<&str>) -> Result<Config, figment::Error> {
        let env_prefix = match prefix {
            Some(prefix) => format!("{}_ASTRIA_CONDUCTOR_", prefix),
            None => "ASTRIA_CONDUCTOR_".to_string(),
        };
        Figment::new()
            .merge(Env::prefixed("RUST_").split("_").only(&["log"]))
            .merge(Env::prefixed(env_prefix.as_str()))
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

    fn populate_environment_from_example(jail: &mut Jail, env_prefix: Option<&str>) {
        static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[[:space:]]+").unwrap());
        static RE_END: Lazy<Regex> = Lazy::new(|| Regex::new(r"[[:space:]]+$").unwrap());
        for line in EXAMPLE_ENV.lines() {
            if let Some((key, val)) = line.trim().split_once('=') {
                if RE_END.is_match(key) || RE_START.is_match(val) {
                    panic!("env vars must not contain spaces in assignment\n{line}");
                }
                let prefixed_key = format!("{}_{}", env_prefix.unwrap_or(""), key);
                jail.set_env(prefixed_key, val);
            }
        }
    }

    #[test]
    fn ensure_example_env_is_in_sync() {
        let env_prefix = "TESTTEST";
        Jail::expect_with(|jail| {
            populate_environment_from_example(jail, Some(env_prefix));
            Config::from_environment(Some(env_prefix)).unwrap();
            Ok(())
        });
    }

    #[test]
    #[should_panic]
    fn extra_env_vars_are_rejected() {
        let env_prefix = "TESTTEST";
        Jail::expect_with(|jail| {
            populate_environment_from_example(jail, Some(env_prefix));
            jail.set_env("TESTTEST_ASTRIA_CONDUCTOR_FOOBAR", "BAZ");
            Config::from_environment(Some(env_prefix)).unwrap();
            Ok(())
        });
    }
}
