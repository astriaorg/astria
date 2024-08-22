//! The conductor configuration.

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum CommitLevel {
    FirmOnly,
    SoftOnly,
    SoftAndFirm,
}

impl CommitLevel {
    pub(crate) fn is_with_firm(self) -> bool {
        matches!(self, Self::FirmOnly | Self::SoftAndFirm)
    }

    pub(crate) fn is_with_soft(self) -> bool {
        matches!(self, Self::SoftOnly | Self::SoftAndFirm)
    }
}

impl std::fmt::Display for CommitLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CommitLevel::SoftOnly => "soft",
            CommitLevel::FirmOnly => "firm",
            CommitLevel::SoftAndFirm => "soft-and-firm",
        };
        f.write_str(s)
    }
}

// Allowed `struct_excessive_bools` because this is used as a container
// for deserialization. Making this a builder-pattern is not actionable.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// The block time of Celestia network in milliseconds.
    pub celestia_block_time_ms: u64,

    /// URL of the Celestia Node HTTP RPC
    pub celestia_node_http_url: String,

    /// Disables using the bearer token auth header for the Celestia jsonrpc
    pub no_celestia_auth: bool,

    /// The JWT bearer token supplied with each jsonrpc call
    pub celestia_bearer_token: String,

    /// URL of the Sequencer Cometbft gRPC service.
    pub sequencer_grpc_url: String,

    /// URL of the Sequencer Cometbft HTTP RPC.
    pub sequencer_cometbft_url: String,

    pub sequencer_block_time_ms: u64,

    /// The number of requests per second that will be sent to Sequencer.
    pub sequencer_requests_per_second: u32,

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
    use super::{
        CommitLevel,
        Config,
    };

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn example_env_config_is_up_to_date() {
        config::tests::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[test]
    fn do_commit_levels_correctly_report_mode() {
        use CommitLevel::{
            FirmOnly,
            SoftAndFirm,
            SoftOnly,
        };

        assert!(FirmOnly.is_with_firm());
        assert!(!FirmOnly.is_with_soft());

        assert!(!SoftOnly.is_with_firm());
        assert!(SoftOnly.is_with_soft());

        assert!(SoftAndFirm.is_with_firm());
        assert!(SoftAndFirm.is_with_soft());
    }
}
