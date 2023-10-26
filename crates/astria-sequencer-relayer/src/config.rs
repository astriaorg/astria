use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
/// The single config for creating an astria-sequencer-relayer service.
#[serde(deny_unknown_fields)]
pub struct Config {
    pub sequencer_endpoint: String,
    pub celestia_endpoint: String,
    pub celestia_bearer_token: String,
    pub gas_limit: u64,
    pub block_time: u64,
    pub disable_relay_all: bool,
    pub validator_key_file: Option<String>,
    pub rpc_port: u16,
    pub log: String,
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_SEQUENCER_RELAYER_";
}

#[cfg(test)]
mod tests {
    use super::Config;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn example_env_config_is_up_to_date() {
        config::tests::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[test]
    fn config_should_reject_unknown_var() {
        config::tests::config_should_reject_unknown_var::<Config>(EXAMPLE_ENV);
    }
}
