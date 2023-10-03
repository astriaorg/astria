use astria_config_derive::astria_config;
use serde::{
    Deserialize,
    Serialize,
};

/// The single config for creating an astria-sequencer-relayer service.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[astria_config(ASTRIA_SEQUENCER_RELAYER_)]
pub struct Config {
    pub sequencer_endpoint: String,
    pub celestia_endpoint: String,
    pub celestia_bearer_token: String,
    pub gas_limit: u64,
    pub block_time: u64,
    pub validator_key_file: String,
    pub rpc_port: u16,
    pub log: String,
}

#[cfg(test)]
mod test {
    use astria_utils::{
        config_test_suite_failing,
        config_test_suite_passing,
    };

    use super::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");
    const ENV_PREFIX: &str = "ASTRIA_SEQUENCER_RELAYER_";

    #[test]
    fn test_config_passing() {
        config_test_suite_passing::<Config>(ENV_PREFIX, EXAMPLE_ENV);
    }

    #[test]
    #[should_panic]
    fn test_config_failing() {
        config_test_suite_failing::<Config>(ENV_PREFIX, EXAMPLE_ENV);
    }
}
