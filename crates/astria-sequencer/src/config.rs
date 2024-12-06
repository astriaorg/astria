use std::path::PathBuf;

use serde::{
    Deserialize,
    Serialize,
};

#[expect(
    clippy::struct_excessive_bools,
    reason = "this is used as a container for deserialization. Making this a builder-pattern is \
              not actionable"
)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// The endpoint on which Sequencer will listen for ABCI requests
    pub listen_addr: String,
    /// The path to penumbra storage db.
    pub db_filepath: PathBuf,
    /// Log level: debug, info, warn, or error
    pub log: String,
    /// The gRPC endpoint
    pub grpc_addr: String,
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
    /// The maximum number of transactions that can be parked in the mempool.
    pub mempool_parked_max_tx_count: usize,
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_SEQUENCER_";
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
