use astria_config::astria_config;
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
    use astria_config::{
        config_test_suite_test_should_fail_with_bad_prefix,
        config_test_suite_test_should_populate_config_with_env_vars,
    };

    use super::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn test_config_passing() {
        config_test_suite_test_should_populate_config_with_env_vars::<Config>(EXAMPLE_ENV);
    }

    #[test]
    #[should_panic]
    fn test_config_failing() {
        config_test_suite_test_should_fail_with_bad_prefix::<Config>(EXAMPLE_ENV);
    }
}
