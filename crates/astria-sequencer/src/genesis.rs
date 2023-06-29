use std::{
    fs::File,
    path::Path,
};

use anyhow::Context;
use serde::Deserialize;

use crate::accounts::types::{
    Address,
    Balance,
};

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
    #[serde(with = "hex::serde")]
    pub(crate) address: Address,
    pub(crate) balance: Balance,
}
