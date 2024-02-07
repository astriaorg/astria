use astria_core::sequencer::v1alpha1::{
    asset,
    Address,
};
use serde::{
    Deserialize,
    Deserializer,
};

/// The genesis state for the application.
#[derive(Debug, Deserialize)]
pub(crate) struct GenesisState {
    pub(crate) accounts: Vec<Account>,
    #[serde(deserialize_with = "deserialize_address")]
    pub(crate) authority_sudo_address: Address,
    #[serde(deserialize_with = "deserialize_address")]
    pub(crate) ibc_sudo_address: Address,
    pub(crate) native_asset_base_denomination: String,
    #[serde(deserialize_with = "deserialize_assets")]
    pub(crate) allowed_fee_assets: Vec<asset::Denom>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Account {
    #[serde(deserialize_with = "deserialize_address")]
    pub(crate) address: Address,
    pub(crate) balance: u128,
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

fn deserialize_assets<'de, D>(deserializer: D) -> Result<Vec<asset::Denom>, D::Error>
where
    D: Deserializer<'de>,
{
    let strings: Vec<String> = serde::Deserialize::deserialize(deserializer)?;
    Ok(strings.into_iter().map(|s| asset::Denom::from(s)).collect())
}
