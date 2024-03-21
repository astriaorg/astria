use std::path::PathBuf;

use serde::{
    Deserialize,
    Serialize,
};

// Allowed `struct_excessive_bools` because this is used as a container
// for deserialization. Making this a builder-pattern is not actionable.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
/// The single config for creating an astria-sequencer-relayer service.
pub struct Config {
    pub cometbft_endpoint: String,
    pub sequencer_grpc_endpoint: String,
    pub celestia_endpoint: String,
    pub celestia_bearer_token: String,
    pub block_time: u64,
    pub relay_only_validator_key_blocks: bool,
    #[serde(default)]
    pub validator_key_file: String,
    // The socket address at which sequencer relayer will server healthz, readyz, and status calls.
    pub api_addr: String,
    pub log: String,
    /// Forces writing trace data to stdout no matter if connected to a tty or not.
    pub force_stdout: bool,
    /// Disables writing trace data to an opentelemetry endpoint.
    pub no_otel: bool,
    /// Set to true to disable the metrics server
    pub no_metrics: bool,
    /// The endpoint which will be listened on for serving prometheus metrics
    pub metrics_http_listener_addr: String,
    /// Writes a human readable format to stdout instead of JSON formatted OTEL trace data.
    pub pretty_print: bool,
    /// The path to which relayer will write its state prior to submitting to Celestia.
    pub pre_submit_path: PathBuf,
    /// The path to which relayer will write its state after submitting to Celestia.
    pub post_submit_path: PathBuf,
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
}
