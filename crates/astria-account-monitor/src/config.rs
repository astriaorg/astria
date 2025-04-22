use std::{
    fmt::Display,
    str::FromStr,
};

use astria_eyre::eyre;
use sequencer_client::Address;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
/// The high-level config for creating an astria-account-monitor service.
pub struct Config {
    /// Log level. One of debug, info, warn, or error
    pub log: String,

    /// Address of the ABCI server for the sequencer chain
    pub sequencer_abci_endpoint: String,

    /// The chain ID of the sequencer chain
    pub sequencer_chain_id: String,

    /// The addresses of the sequencer chain to monitor.
    pub sequencer_accounts: Vec<Account>,

    /// The asset ID of the sequencer chain to monitor.
    pub sequencer_asset: String,

    /// Sequencer block time in milliseconds
    pub query_interval_ms: u64,

    /// Forces writing trace data to stdout no matter if connected to a tty or not.
    pub force_stdout: bool,

    /// Disables writing trace data to an opentelemetry endpoint.
    pub no_otel: bool,

    /// The endpoint which will be listened on for serving prometheus metrics
    pub metrics_http_listener_addr: String,
}

#[derive(Debug, Clone)]
pub struct Account {
    /// The address of the account to monitor.
    pub address: Address,
}

impl FromStr for Account {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let address = s
            .parse()
            .map_err(|e| eyre::eyre!("failed to parse account address: {e}"))?;
        Ok(Self {
            address,
        })
    }
}

impl Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.address)
    }
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_ACCOUNT_MONITOR_";
}

impl Config {
    /// Returns a list of addresses from a comma-separated string.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing any of the addresses fails.
    pub fn parse_accounts(&self) -> eyre::Result<Vec<Account>> {
        Ok(self.sequencer_accounts.clone())
    }
}

impl<'de> Deserialize<'de> for Account {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
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
