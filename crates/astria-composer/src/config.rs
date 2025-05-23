use std::{
    collections::HashMap,
    net::SocketAddr,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::rollup::{
    ParseError,
    Rollup,
};

#[derive(Debug, Deserialize, Serialize)]
/// The high-level config for creating an astria-composer service.
pub struct Config {
    /// Log level. One of debug, info, warn, or error
    pub log: String,

    /// Address of the API server
    pub api_listen_addr: SocketAddr,

    /// Address of the ABCI server for the sequencer chain
    pub sequencer_abci_endpoint: String,

    /// Address of the GRPC server for the sequencer chain
    pub sequencer_grpc_endpoint: String,

    /// The chain ID of the sequencer chain
    pub sequencer_chain_id: String,

    /// A list of `<rollup_name>::<url>` pairs
    pub rollups: String,

    /// Path to private key for the sequencer account used for signing transactions
    pub private_key_file: String,

    /// The address prefix to use when constructing sequencer addresses using the signing key.
    pub sequencer_address_prefix: String,

    /// Sequencer block time in milliseconds
    #[serde(alias = "max_submit_interval_ms")]
    pub block_time_ms: u64,

    /// Max bytes to encode into a single sequencer transaction, not including signature,
    /// public key, nonce. This is the sum of the sizes of all the sequence actions.
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

    /// The address at which the gRPC server is listening
    pub grpc_addr: SocketAddr,

    /// The IBC asset to pay for transactions submiited to the sequencer.
    pub fee_asset: astria_core::primitive::v1::asset::Denom,
}

impl Config {
    /// Returns a map of rollup names to rollup URLs.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    pub fn parse_rollups(&self) -> Result<HashMap<String, String>, ParseError> {
        self.rollups
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| Rollup::parse(s).map(Rollup::into_parts))
            .collect::<Result<HashMap<_, _>, _>>()
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
