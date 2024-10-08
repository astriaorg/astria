use std::borrow::Cow;

use astria_core::primitive::v1::asset::IbcPrefixed;
use astria_eyre::eyre::{
    OptionExt as _,
    Result,
    WrapErr as _,
};

use crate::{
    accounts::AddressBytes,
    storage::keys::{
        AccountPrefixer,
        Asset,
    },
};

pub(in crate::accounts) const TRANSFER_BASE_FEE: &str = "accounts/transfer_base_fee";
const COMPONENT_PREFIX: &str = "accounts/";
const BALANCE_PREFIX: &str = "balance/";
const NONCE: &str = "nonce";

/// Example: `accounts/gGhH....zZ4=/balance/`.
///                   |base64 chars|
pub(in crate::accounts) fn balance_prefix<TAddress: AddressBytes>(address: &TAddress) -> String {
    format!(
        "{}/{BALANCE_PREFIX}",
        AccountPrefixer::new(COMPONENT_PREFIX, address)
    )
}

/// Example: `accounts/gGhH....zZ4=/balance/0202....0202`.
///                   |base64 chars|       |64 hex chars|
pub(in crate::accounts) fn balance<'a, TAddress, TAsset>(
    address: &TAddress,
    asset: &'a TAsset,
) -> String
where
    TAddress: AddressBytes,
    &'a TAsset: Into<Cow<'a, IbcPrefixed>>,
{
    format!(
        "{}/{BALANCE_PREFIX}{}",
        AccountPrefixer::new(COMPONENT_PREFIX, address),
        Asset::from(asset)
    )
}

/// Example: `accounts/gGhH....zZ4=/nonce`.
///                   |base64 chars|
pub(in crate::accounts) fn nonce<TAddress: AddressBytes>(address: &TAddress) -> String {
    format!(
        "{}/{NONCE}",
        AccountPrefixer::new(COMPONENT_PREFIX, address)
    )
}

pub(in crate::accounts) fn extract_asset_from_key(key: &str) -> Result<IbcPrefixed> {
    Ok(key
        .strip_prefix(COMPONENT_PREFIX)
        .and_then(|s| s.split_once(BALANCE_PREFIX).map(|(_, asset)| asset))
        .ok_or_eyre("failed to strip prefix from account balance key")?
        .parse::<Asset>()
        .wrap_err("failed to parse storage key suffix as address hunk")?
        .get())
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::{
        asset::Denom,
        Address,
    };

    use super::*;

    fn address() -> Address {
        "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
            .parse()
            .unwrap()
    }

    fn asset() -> Denom {
        "an/asset/with/a/prefix".parse().unwrap()
    }

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(TRANSFER_BASE_FEE);
        insta::assert_snapshot!(balance(&address(), &asset()));
        insta::assert_snapshot!(nonce(&address()));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(TRANSFER_BASE_FEE.starts_with(COMPONENT_PREFIX));
        assert!(balance(&address(), &asset()).starts_with(COMPONENT_PREFIX));
        assert!(nonce(&address()).starts_with(COMPONENT_PREFIX));
    }

    #[test]
    fn balance_prefix_should_be_prefix_of_balance_key() {
        assert!(balance(&address(), &asset()).starts_with(&balance_prefix(&address())));
    }

    #[test]
    fn should_extract_asset_from_key() {
        let asset = IbcPrefixed::new([2; 32]);
        let key = balance(&[1; 20], &asset);
        let recovered_asset = extract_asset_from_key(&key).unwrap();
        assert_eq!(asset, recovered_asset);
    }
}
