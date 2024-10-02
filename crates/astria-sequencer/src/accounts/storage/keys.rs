use std::borrow::Cow;

use astria_core::primitive::v1::asset::IbcPrefixed;
use astria_eyre::eyre::{
    ContextCompat as _,
    Result,
    WrapErr as _,
};

use crate::{
    accounts::AddressBytes,
    storage::keys::{
        AddressPrefixer,
        Asset,
    },
};

pub(in crate::accounts) const TRANSFER_BASE_FEE_KEY: &str = "accounts/transfer_base_fee";
const COMPONENT_PREFIX: &str = "accounts/";
const BALANCE_PREFIX: &str = "balance/";
const NONCE: &str = "nonce";

/// Example: `accounts/0101....0101/balance/`.
///                   |40 hex chars|
pub(in crate::accounts) fn balance_prefix<TAddress: AddressBytes>(address: &TAddress) -> String {
    format!(
        "{}/{BALANCE_PREFIX}",
        AddressPrefixer::new(COMPONENT_PREFIX, address)
    )
}

/// Example: `accounts/0101....0101/balance/0202....0202`.
///                   |40 hex chars|       |64 hex chars|
pub(in crate::accounts) fn balance_key<'a, TAddress, TAsset>(
    address: &TAddress,
    asset: &'a TAsset,
) -> String
where
    TAddress: AddressBytes,
    &'a TAsset: Into<Cow<'a, IbcPrefixed>>,
{
    format!(
        "{}/{BALANCE_PREFIX}{}",
        AddressPrefixer::new(COMPONENT_PREFIX, address),
        Asset::from(asset)
    )
}

/// Example: `accounts/0101....0101/nonce`.
///                   |40 hex chars|
pub(in crate::accounts) fn nonce_key<TAddress: AddressBytes>(address: &TAddress) -> String {
    format!(
        "{}/{NONCE}",
        AddressPrefixer::new(COMPONENT_PREFIX, address)
    )
}

pub(in crate::accounts) fn extract_asset_from_key(key: &str) -> Result<IbcPrefixed> {
    Ok(key
        .strip_prefix(COMPONENT_PREFIX)
        .and_then(|s| s.split_once(BALANCE_PREFIX).map(|(_, asset)| asset))
        .wrap_err("failed to strip prefix from account balance key")?
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
        insta::assert_snapshot!(TRANSFER_BASE_FEE_KEY);
        insta::assert_snapshot!(balance_key(&address(), &asset()));
        insta::assert_snapshot!(nonce_key(&address()));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(TRANSFER_BASE_FEE_KEY.starts_with(COMPONENT_PREFIX));
        assert!(balance_key(&address(), &asset()).starts_with(COMPONENT_PREFIX));
        assert!(nonce_key(&address()).starts_with(COMPONENT_PREFIX));
    }

    #[test]
    fn balance_prefix_should_be_prefix_of_balance_key() {
        assert!(balance_key(&address(), &asset()).starts_with(&balance_prefix(&address())));
    }

    #[test]
    fn should_extract_asset_from_key() {
        let asset = IbcPrefixed::new([2; 32]);
        let key = balance_key(&[1; 20], &asset);
        let recovered_asset = extract_asset_from_key(&key).unwrap();
        assert_eq!(asset, recovered_asset);
    }
}
