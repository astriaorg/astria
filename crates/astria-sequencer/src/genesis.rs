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

#[derive(Debug, Deserialize)]
pub(crate) struct Account {
    pub(crate) address: Address,
    pub(crate) balance: Balance,
}
