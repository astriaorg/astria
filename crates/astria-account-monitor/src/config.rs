use astria_eyre::eyre;
use sequencer_client::Address;
use serde::{
    Deserialize,
    Serialize,
};

#[expect(
    clippy::struct_excessive_bools,
    reason = "this is a config, may have many boolean values"
)]
#[derive(Debug, Deserialize, Serialize)]
/// The high-level config for creating an astria-account-monitor service.
pub struct Config {
    /// Log level. One of debug, info, warn, or error
    pub log: String,

    /// Address of the ABCI server for the sequencer chain
    pub sequencer_abci_endpoint: String,

    /// The chain ID of the sequencer chain
    pub sequencer_chain_id: String,

    /// The address prefix to use when constructing sequencer addresses using the signing key.
    pub sequencer_address_prefix: String,

    /// The addresses of the sequencer chain to monitor.
    pub sequencer_accounts: String,

    pub sequencer_bridge_accounts: String,
    /// The asset ID of the sequencer chain to monitor.
    pub sequencer_asset: String,

    /// Sequencer block time in milliseconds
    #[serde(alias = "query_interval_ms")]
    pub block_time_ms: u64,

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
    const PREFIX: &'static str = "ASTRIA_ACCOUNT_MONITOR_";
}

impl Config {
    pub fn parse_account(&self, account: &str) -> eyre::Result<Address> {
        let address = account
            .parse()
            .map_err(|e| eyre::eyre!("failed to parse account address: {e}"))?;
        Ok(address)
    }

    pub fn parse_accounts(&self) -> eyre::Result<Vec<Address>> {
        let accounts = self
            .sequencer_accounts
            .split(',')
            .map(|account| self.parse_account(account))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(accounts)
    }

    pub fn parse_bridge_accounts(&self) -> eyre::Result<Vec<Address>> {
        let accounts = self
            .sequencer_bridge_accounts
            .split(',')
            .map(|account| self.parse_account(account))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(accounts)
    }
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
