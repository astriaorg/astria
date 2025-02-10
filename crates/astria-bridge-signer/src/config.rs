use serde::{
    Deserialize,
    Serialize,
};

#[expect(
    clippy::struct_excessive_bools,
    reason = "This is used as a container for deserialization. Making this a builder-pattern is \
              not actionable"
)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Config {
    /// The address of the grpc endpoint.
    pub grpc_endpoint: String,
    /// The path to the json-encoded frost secret key package.
    pub frost_secret_key_package_path: String,
    /// Rollup EVM node RPC endpoint.
    pub rollup_rpc_endpoint: String,
    /// Log filter directives.
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
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_BRIDGE_SIGNER_";
}
