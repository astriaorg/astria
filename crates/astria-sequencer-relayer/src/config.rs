use config::astria_config;
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
    use super::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn example_env_config_is_up_to_date() {
        config::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[test]
    #[should_panic]
    fn config_should_reject_unknown_var() {
        config::config_should_reject_unknown_var::<Config>(EXAMPLE_ENV);
    }
}
