//! The conductor configuration.

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CommitLevel {
    SoftOnly,
    FirmOnly,
    SoftAndFirm,
}

impl CommitLevel {
    pub(crate) fn is_soft_only(&self) -> bool {
        matches!(self, Self::SoftOnly)
    }

    pub(crate) fn is_firm_only(&self) -> bool {
        matches!(self, Self::FirmOnly)
    }
}

// Allowed `struct_excessive_bools` because this is used as a container
// for deserialization. Making this a builder-pattern is not actionable.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// URL of the Celestia Node
    pub celestia_node_url: String,

    /// The JWT bearer token supplied with each jsonrpc call
    pub celestia_bearer_token: String,

    /// URL of the sequencer cometbft websocket
    pub sequencer_url: String,

    /// Address of the RPC server for execution
    pub execution_rpc_url: String,

    /// log directive to use for telemetry.
    pub log: String,

    /// The execution commit level used for controlling how blocks are sent to
    /// the execution layer.
    pub execution_commit_level: CommitLevel,

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
    const PREFIX: &'static str = "ASTRIA_CONDUCTOR_";
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
