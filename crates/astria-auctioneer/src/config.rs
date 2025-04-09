use astria_core::primitive::v1::asset;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
/// The single config for creating an astria-auctioneer service.
pub struct Config {
    /// The endpoint for the sequencer gRPC service used for the proposed block stream
    pub sequencer_grpc_endpoint: String,
    /// The endpoint for the sequencer ABCI service used for submitting the auction winner
    /// transaction
    pub sequencer_abci_endpoint: String,
    /// The chain ID for the sequencer network
    pub sequencer_chain_id: String,
    /// The file path for the private key used to sign sequencer transactions with the auction
    /// results
    pub sequencer_private_key_path: String,
    // The address prefix to use when constructing sequencer addresses using the signing key.
    pub sequencer_address_prefix: String,
    // The fee asset denomination to use for the sequnecer transactions.
    pub fee_asset_denomination: asset::Denom,
    /// The endpoint for the rollup gRPC service used for the optimistic execution and bundle
    /// streams
    pub rollup_grpc_endpoint: String,
    /// The rollup ID used to filter the proposed blocks stream
    pub rollup_id: String,
    /// The amount of time in miliseconds to wait after a commit before closing the auction for
    /// bids and submitting the result to the sequencer.
    pub latency_margin_ms: u64,
    /// Log level for the service.
    pub log: String,
    /// Forces writing trace data to stdout no matter if connected to a tty or not.
    pub force_stdout: bool,
    /// Disables writing trace data to an opentelemetry endpoint.
    pub no_otel: bool,
    /// Set to true to disable the metrics server
    pub no_metrics: bool,
    /// The endpoint which will be listened on for serving prometheus metrics
    pub metrics_http_listener_addr: String,
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_AUCTIONEER_";
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
