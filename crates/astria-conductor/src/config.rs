//! The conductor configuration.

use figment::{
    providers::Env,
    Figment,
};
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CommitLevel {
    SoftOnly,
    FirmOnly,
    SoftAndFirm,
}

impl CommitLevel {
    pub fn is_soft_only(&self) -> bool {
        matches!(self, Self::SoftOnly)
    }

    pub fn is_firm_only(&self) -> bool {
        matches!(self, Self::FirmOnly)
    }
}

pub fn get() -> Result<Config, figment::Error> {
    Config::from_environment("ASTRIA_CONDUCTOR_")
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// URL of the Celestia Node
    pub celestia_node_url: String,

    /// The JWT bearer token supplied with each jsonrpc call
    pub celestia_bearer_token: String,

    /// URL of the sequencer cometbft websocket
    pub sequencer_url: String,

    /// Chain ID that we want to work in
    pub chain_id: String,

    /// Address of the RPC server for execution
    pub execution_rpc_url: String,

    /// log directive to use for telemetry.
    pub log: String,

    /// Choose to execute empty blocks or not
    pub disable_empty_block_execution: bool,

    /// The Sequencer block height that the rollup genesis block was in
    pub initial_sequencer_block_height: u32,

    /// The execution commit level used for controlling how blocks are sent to
    /// the execution layer.
    pub execution_commit_level: CommitLevel,
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
                if RE_END.is_match(key) || RE_START.is_match(val) {
                    panic!("env vars must not contain spaces in assignment\n{line}");
                }
                let prefixed_key = format!("{}_{}", test_envar_prefix, key);
                jail.set_env(prefixed_key, val);
            }
        }
    }

    #[test]
    fn ensure_example_env_is_in_sync() {
        let test_envar_prefix = "TESTTEST";
        let full_envar_prefix = format!("{}_{}", test_envar_prefix, "ASTRIA_CONDUCTOR_");
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
        let full_envar_prefix = format!("{}_{}", test_envar_prefix, "ASTRIA_CONDUCTOR_");
        Jail::expect_with(|jail| {
            populate_environment_from_example(jail, test_envar_prefix);
            jail.set_env("TESTTEST_ASTRIA_CONDUCTOR_FOOBAR", "BAZ");
            Config::from_environment(full_envar_prefix.as_str()).unwrap();
            Ok(())
        });
    }
}
