use std::{
    fs::File,
    path::Path,
};

use anyhow::Context;
use astria_proto::native::sequencer::Address;
use serde::{
    Deserialize,
    Deserializer,
};

use crate::accounts::types::Balance;

/// The genesis state for the application.
#[derive(Debug, Deserialize, Default)]
pub(crate) struct GenesisState {
    pub(crate) accounts: Vec<Account>,
}

impl GenesisState {
    pub(crate) fn from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let file = File::open(path).context("failed to open file with genesis state")?;
        serde_json::from_reader(&file).context("failed deserializing genesis state from file")
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Account {
    #[serde(deserialize_with = "deserialize_address")]
    pub(crate) address: Address,
    pub(crate) balance: Balance,
}

fn deserialize_address<'de, D>(deserializer: D) -> Result<Address, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error as _;
    let bytes: Vec<u8> = hex::serde::deserialize(deserializer)?;
    Address::try_from_slice(&bytes)
        .map_err(|e| D::Error::custom(format!("failed constructing address from bytes: {e}")))
}
