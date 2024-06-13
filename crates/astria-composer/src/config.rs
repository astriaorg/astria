use std::{
    collections::HashMap,
    net::SocketAddr,
};

use astria_eyre::eyre::WrapErr;
use serde::{
    Deserialize,
    Serialize,
};

use crate::rollup::Rollup;

// this is a config, may have many boolean values
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Deserialize, Serialize)]
/// The high-level config for creating an astria-composer service.
pub struct Config {
    /// Log level. One of debug, info, warn, or error
    pub log: String,

    /// Address of the API server
    pub api_listen_addr: SocketAddr,

    /// Address of the RPC server for the sequencer chain
    pub sequencer_url: String,

    /// The chain ID of the sequencer chain
    pub sequencer_chain_id: String,

    /// A list of `<rollup_name>::<url>` pairs
    pub rollups: String,

    /// Path to private key for the sequencer account used for signing transactions
    pub private_key_file: String,

    /// Sequencer block time in milliseconds
    #[serde(alias = "max_submit_interval_ms")]
    pub block_time_ms: u64,

    /// Max bytes to encode into a single sequencer `SignedTransaction`, not including signature,
    /// public key, nonce. This is the sum of the sizes of all the `SequenceAction`s
    pub max_bytes_per_bundle: usize,

    /// Max amount of `SizedBundle`s to allow to accrue in the `BundleFactory`'s finished queue.
    pub bundle_queue_capacity: usize,

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

    /// The address at which the gRPC server is listening
    pub grpc_addr: SocketAddr,
}

impl Config {
    pub(crate) fn parse_rollups(&self) -> astria_eyre::eyre::Result<HashMap<String, String>> {
        self.rollups
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| Rollup::parse(s).map(Rollup::into_parts))
            .collect::<Result<HashMap<_, _>, _>>()
            .wrap_err("failed parsing provided <rollup_name>::<url> pairs as rollups")
    }
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_COMPOSER_";
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
