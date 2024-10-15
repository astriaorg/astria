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
pub(in crate::fees) const ALLOWED_ASSET_PREFIX: &str = "fees/allowed_asset/";

pub(in crate::fees) fn allowed_asset<'a, TAsset>(asset: &'a TAsset) -> String
where
    &'a TAsset: Into<Cow<'a, IbcPrefixed>>,
{
    format!("{ALLOWED_ASSET_PREFIX}{}", Asset::from(asset))
}

pub(in crate::fees) fn extract_asset_from_allowed_asset_key(
    key: &str,
) -> eyre::Result<IbcPrefixed> {
    extract_asset_from_key(key, ALLOWED_ASSET_PREFIX)
        .wrap_err("failed to extract asset from fee asset key")
}

fn extract_asset_from_key(key: &str, prefix: &str) -> eyre::Result<IbcPrefixed> {
    let suffix = key
        .strip_prefix(prefix)
        .ok_or_else(|| eyre!("key `{key}` did not have prefix `{prefix}`"))?;
    suffix.parse().wrap_err_with(|| {
        format!("failed to parse suffix `{suffix}` of key `{key}` as an ibc-prefixed asset",)
    })
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::asset::Denom;
    use insta::assert_snapshot;

    use super::*;

    const COMPONENT_PREFIX: &str = "fees/";

    fn test_asset() -> Denom {
        "an/asset/with/a/prefix".parse().unwrap()
    }

    #[test]
    fn keys_should_not_change() {
        // NOTE: This helper struct is just to avoid having 14 snapshot files to contend with.
        // NOTE: `BLOCK` is only used in the ephemeral store, so isn't included here.
        assert_snapshot!("bridge_lock_fees_key", BRIDGE_LOCK);
        assert_snapshot!("bridge_sudo_change_fees_key", BRIDGE_SUDO_CHANGE);
        assert_snapshot!("bridge_unlock_fees_key", BRIDGE_UNLOCK);
        assert_snapshot!("fee_asset_change_fees_key", FEE_ASSET_CHANGE);
        assert_snapshot!("allowed_asset_prefix", ALLOWED_ASSET_PREFIX);
        assert_snapshot!("fee_change_fees_key", FEE_CHANGE);
        assert_snapshot!("ibc_relay_fees_key", IBC_RELAY);
        assert_snapshot!("ibc_relayer_change_fees_key", IBC_RELAYER_CHANGE);
        assert_snapshot!("ibc_sudo_change_fees_key", IBC_SUDO_CHANGE);
        assert_snapshot!("ics20_withdrawal_fees_key", ICS20_WITHDRAWAL);
        assert_snapshot!("init_bridge_account_fees_key", INIT_BRIDGE_ACCOUNT);
        assert_snapshot!("sequence_fees_key", SEQUENCE);
        assert_snapshot!("sudo_address_change_fees_key", SUDO_ADDRESS_CHANGE);
        assert_snapshot!("transer_fees_key", TRANSFER);
        assert_snapshot!("validator_update_fees_key", VALIDATOR_UPDATE);
        assert_snapshot!("allowed_asset_key", allowed_asset(&test_asset()));
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
        assert!(ALLOWED_ASSET_PREFIX.starts_with(COMPONENT_PREFIX));
        assert!(allowed_asset(&test_asset()).starts_with(COMPONENT_PREFIX));
    }

    #[test]
    fn prefixes_should_be_prefixes_of_relevant_keys() {
        assert!(allowed_asset(&test_asset()).starts_with(ALLOWED_ASSET_PREFIX));
    }

    #[test]
    fn should_extract_asset_from_key() {
        let asset = IbcPrefixed::new([1; 32]);

        let key = allowed_asset(&asset);
        let recovered_asset = extract_asset_from_allowed_asset_key(&key).unwrap();
        assert_eq!(asset, recovered_asset);
    }
}
