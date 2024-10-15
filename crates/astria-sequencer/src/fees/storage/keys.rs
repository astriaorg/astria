use std::borrow::Cow;

use astria_core::primitive::v1::asset::IbcPrefixed;
use astria_eyre::eyre::{
    self,
    eyre,
    Context as _,
};

use crate::storage::keys::Asset;

pub(in crate::fees) const TRANSFER: &str = "fees/transfer";
pub(in crate::fees) const SEQUENCE: &str = "fees/sequence";
pub(in crate::fees) const ICS20_WITHDRAWAL: &str = "fees/ics20_withdrawal";
pub(in crate::fees) const INIT_BRIDGE_ACCOUNT: &str = "fees/init_bridge_account";
pub(in crate::fees) const BRIDGE_LOCK: &str = "fees/bridge_lock";
pub(in crate::fees) const BRIDGE_UNLOCK: &str = "fees/bridge_unlock";
pub(in crate::fees) const BRIDGE_SUDO_CHANGE: &str = "fees/bridge_sudo_change";
pub(in crate::fees) const IBC_RELAY: &str = "fees/ibc_relay";
pub(in crate::fees) const VALIDATOR_UPDATE: &str = "fees/validator_update";
pub(in crate::fees) const FEE_ASSET_CHANGE: &str = "fees/fee_asset_change";
pub(in crate::fees) const FEE_CHANGE: &str = "fees/fee_change";
pub(in crate::fees) const IBC_RELAYER_CHANGE: &str = "fees/ibc_relayer_change";
pub(in crate::fees) const SUDO_ADDRESS_CHANGE: &str = "fees/sudo_address_change";
pub(in crate::fees) const IBC_SUDO_CHANGE: &str = "fees/ibc_sudo_change";
pub(in crate::fees) const BLOCK: &str = "fees/block"; // NOTE: `BLOCK` is only used in the ephemeral store.
pub(in crate::fees) const FEE_ASSET_PREFIX: &str = "fees/fee_asset/";

pub(in crate::fees) fn fee_asset<'a, TAsset>(asset: &'a TAsset) -> String
where
    &'a TAsset: Into<Cow<'a, IbcPrefixed>>,
{
    format!("{FEE_ASSET_PREFIX}{}", Asset::from(asset))
}

pub(in crate::fees) fn extract_asset_from_fee_asset_key(key: &[u8]) -> eyre::Result<IbcPrefixed> {
    extract_asset_from_key(key, FEE_ASSET_PREFIX)
        .wrap_err("failed to extract asset from fee asset key")
}

fn extract_asset_from_key(key: &[u8], prefix: &str) -> eyre::Result<IbcPrefixed> {
    let key_str = std::str::from_utf8(key)
        .wrap_err_with(|| format!("key `{}` not valid utf8", telemetry::display::hex(key),))?;
    let suffix = key_str
        .strip_prefix(prefix)
        .ok_or_else(|| eyre!("key `{key_str}` did not have prefix `{prefix}`"))?;
    suffix.parse().wrap_err_with(|| {
        format!("failed to parse suffix `{suffix}` of key `{key_str}` as an ibc-prefixed asset",)
    })
}

#[cfg(test)]
mod tests {
    use std::fmt::{
        self,
        Display,
        Formatter,
    };

    use astria_core::primitive::v1::asset::Denom;

    use super::*;

    const COMPONENT_PREFIX: &str = "fees/";

    fn test_asset() -> Denom {
        "an/asset/with/a/prefix".parse().unwrap()
    }

    #[test]
    fn keys_should_not_change() {
        // NOTE: This helper struct is just to avoid having 14 snapshot files to contend with.
        // NOTE: `BLOCK` is only used in the ephemeral store, so isn't included here.
        struct Helper;
        impl Display for Helper {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                writeln!(formatter, "{BRIDGE_LOCK}")?;
                writeln!(formatter, "{BRIDGE_SUDO_CHANGE}")?;
                writeln!(formatter, "{BRIDGE_UNLOCK}")?;
                writeln!(formatter, "{FEE_ASSET_CHANGE}")?;
                writeln!(formatter, "{FEE_ASSET_PREFIX}")?;
                writeln!(formatter, "{FEE_CHANGE}")?;
                writeln!(formatter, "{IBC_RELAY}")?;
                writeln!(formatter, "{IBC_RELAYER_CHANGE}")?;
                writeln!(formatter, "{IBC_SUDO_CHANGE}")?;
                writeln!(formatter, "{ICS20_WITHDRAWAL}")?;
                writeln!(formatter, "{INIT_BRIDGE_ACCOUNT}")?;
                writeln!(formatter, "{SEQUENCE}")?;
                writeln!(formatter, "{SUDO_ADDRESS_CHANGE}")?;
                writeln!(formatter, "{TRANSFER}")?;
                writeln!(formatter, "{VALIDATOR_UPDATE}")?;
                Ok(())
            }
        }
        insta::assert_snapshot!(Helper);
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(TRANSFER.starts_with(COMPONENT_PREFIX));
        assert!(SEQUENCE.starts_with(COMPONENT_PREFIX));
        assert!(ICS20_WITHDRAWAL.starts_with(COMPONENT_PREFIX));
        assert!(INIT_BRIDGE_ACCOUNT.starts_with(COMPONENT_PREFIX));
        assert!(BRIDGE_LOCK.starts_with(COMPONENT_PREFIX));
        assert!(BRIDGE_UNLOCK.starts_with(COMPONENT_PREFIX));
        assert!(BRIDGE_SUDO_CHANGE.starts_with(COMPONENT_PREFIX));
        assert!(IBC_RELAY.starts_with(COMPONENT_PREFIX));
        assert!(VALIDATOR_UPDATE.starts_with(COMPONENT_PREFIX));
        assert!(FEE_ASSET_CHANGE.starts_with(COMPONENT_PREFIX));
        assert!(FEE_CHANGE.starts_with(COMPONENT_PREFIX));
        assert!(IBC_RELAYER_CHANGE.starts_with(COMPONENT_PREFIX));
        assert!(SUDO_ADDRESS_CHANGE.starts_with(COMPONENT_PREFIX));
        assert!(IBC_SUDO_CHANGE.starts_with(COMPONENT_PREFIX));
        assert!(FEE_ASSET_PREFIX.starts_with(COMPONENT_PREFIX));
        assert!(fee_asset(&test_asset()).starts_with(COMPONENT_PREFIX));
    }

    #[test]
    fn prefixes_should_be_prefixes_of_relevant_keys() {
        assert!(fee_asset(&test_asset()).starts_with(FEE_ASSET_PREFIX));
    }

    #[test]
    fn should_extract_asset_from_key() {
        let asset = IbcPrefixed::new([1; 32]);

        let key = fee_asset(&asset);
        let recovered_asset = extract_asset_from_fee_asset_key(key.as_bytes()).unwrap();
        assert_eq!(asset, recovered_asset);
    }
}
