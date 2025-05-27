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

    /// The addresses of the sequencer chain to monitor.
    pub accounts: SequencerAccountsToMonitor,

    /// The asset ID of the sequencer chain to monitor.
    pub assets: SequencerAssetsToMonitor,

    /// Sequencer block time in milliseconds
    pub query_interval_ms: u64,

    /// Forces writing trace data to stdout no matter if connected to a tty or not.
    pub force_stdout: bool,

    /// Disables writing trace data to an opentelemetry endpoint.
    pub no_otel: bool,

    /// The endpoint which will be listened on for serving prometheus metrics
    pub metrics_http_listener_addr: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SequencerAssetsToMonitor(Vec<Denom>);

impl SequencerAssetsToMonitor {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Denom> {
        self.0.iter()
    }
}

impl<'de> Deserialize<'de> for SequencerAssetsToMonitor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        if value.is_empty() {
            return Err(serde::de::Error::custom("empty asset list"));
        }

        let assets = value
            .split(',')
            .map(str::trim)
            .map(str::parse)
            .collect::<Result<Vec<Denom>, _>>()
            .map_err(serde::de::Error::custom)?;

        let mut items = HashSet::new();
        if !assets.iter().all(|item| items.insert(item)) {
            return Err(serde::de::Error::custom("duplicate assets"));
        }

        Ok(Self(assets))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SequencerAccountsToMonitor(Vec<Account>);

impl SequencerAccountsToMonitor {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Account> {
        self.0.iter()
    }
}

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
    use astria_core::primitive::v1::asset::Denom;

    use super::Config;
    use crate::config::SequencerAssetsToMonitor;

    const EXAMPLE_ENV: &str = include_str!("../local.env.example");

    #[test]
    fn example_env_config_is_up_to_date() {
        config::tests::example_env_config_is_up_to_date::<Config>(EXAMPLE_ENV);
    }

    #[test]
    fn test_deserialize_multiple_assets() {
        let input = r#""nria,transfer/channel-0/utia""#;
        let result: Result<SequencerAssetsToMonitor, _> = serde_json::from_str(input);

        assert!(result.is_ok());
        let assets = result.unwrap();
        let collected: Vec<&Denom> = assets.iter().collect();

        assert_eq!(collected.len(), 2);
        let asset_strings: Vec<String> = collected
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        assert!(asset_strings.contains(&"nria".to_string()));
        assert!(asset_strings.contains(&"transfer/channel-0/utia".to_string()));
    }
}
