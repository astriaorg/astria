use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::Display,
    hash::Hash,
    str::FromStr,
};

use astria_core::primitive::v1::asset::Denom;
use sequencer_client::Address;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
/// The high-level config for creating an astria-account-monitor service.
pub struct Config {
    /// Log level. One of debug, info, warn, or error
    pub log: String,

    /// Address of the ABCI server for the sequencer chain
    pub sequencer_abci_endpoint: String,

    /// The chain ID of the sequencer chain
    pub sequencer_chain_id: String,

    /// The addresses of the sequencer chain to monitor.
    pub sequencer_accounts: SequencerAccountsToMonitor,

    /// The asset ID of the sequencer chain to monitor.
    pub sequencer_asset: Asset,

    /// Sequencer block time in milliseconds
    pub query_interval_ms: u64,

    /// Forces writing trace data to stdout no matter if connected to a tty or not.
    pub force_stdout: bool,

    /// Disables writing trace data to an opentelemetry endpoint.
    pub no_otel: bool,

    /// The endpoint which will be listened on for serving prometheus metrics
    pub metrics_http_listener_addr: String,
}

#[derive(Debug, Clone, Hash, Serialize)]
pub struct Asset {
    /// The asset ID of the sequencer chain to monitor.
    pub asset: Denom,
}

impl<'de> Deserialize<'de> for Asset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let asset = value.parse().map_err(serde::de::Error::custom)?;

        Ok(Self {
            asset,
        })
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.asset)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SequencerAccountsToMonitor(Vec<Account>);

impl<'de> Deserialize<'de> for SequencerAccountsToMonitor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        if value.is_empty() {
            return Err(serde::de::Error::custom("empty account list"));
        }

        let accounts: Result<Vec<Account>, _> = value
            .split(',')
            .map(str::trim)
            .map(str::parse)
            .collect::<Result<Vec<Account>, _>>();

        let accounts = accounts.map_err(serde::de::Error::custom)?;

        let mut items = HashSet::new();
        if !accounts.iter().all(|item| items.insert(item)) {
            return Err(serde::de::Error::custom("duplicate accounts"));
        }

        Ok(Self(accounts))
    }
}

#[expect(clippy::into_iter_without_iter, reason = "iter() is not needed")]
impl<'a> IntoIterator for &'a SequencerAccountsToMonitor {
    type IntoIter = std::slice::Iter<'a, Account>;
    type Item = &'a Account;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Account {
    /// The address of the account to monitor.
    pub address: Address,
}

impl From<Address> for Account {
    fn from(address: Address) -> Self {
        Self {
            address,
        }
    }
}

impl FromStr for Account {
    type Err = <Address as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let address = s.parse()?;
        Ok(Self {
            address,
        })
    }
}

impl Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.address.fmt(f)
    }
}

impl config::Config for Config {
    const PREFIX: &'static str = "ASTRIA_ACCOUNT_MONITOR_";
}

impl<'de> Deserialize<'de> for Account {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = Cow::<'_, str>::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.address.to_string())
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
