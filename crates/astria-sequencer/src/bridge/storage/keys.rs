use astria_core::primitive::v1::RollupId;
use base64::{
    display::Base64Display,
    engine::general_purpose::URL_SAFE,
};

use crate::{
    accounts::AddressBytes,
    storage::keys::AccountPrefixer,
};

pub(in crate::bridge) const INIT_BRIDGE_ACCOUNT_BASE_FEE: &str = "bridge/init_account_fee";
pub(in crate::bridge) const BRIDGE_LOCK_BYTE_COST_MULTIPLIER: &str =
    "bridge/lock_byte_cost_multiplier";
pub(in crate::bridge) const BRIDGE_SUDO_CHANGE_FEE: &str = "bridge/sudo_change_fee";

pub(in crate::bridge) const BRIDGE_ACCOUNT_PREFIX: &str = "bridge/account/";
const BRIDGE_ACCOUNT_SUDO_PREFIX: &str = "bridge/sudo/";
const BRIDGE_ACCOUNT_WITHDRAWER_PREFIX: &str = "bridge/withdrawer/";

pub(in crate::bridge) const DEPOSITS_EPHEMERAL: &str = "bridge/deposits";
const DEPOSIT_PREFIX: &str = "bridge/deposit/";

/// Example: `bridge/account/gGhH....zZ4=/rollup_id`.
///                         |base64 chars|
pub(in crate::bridge) fn rollup_id<T: AddressBytes>(address: &T) -> String {
    format!(
        "{}/rollup_id",
        AccountPrefixer::new(BRIDGE_ACCOUNT_PREFIX, address)
    )
}

/// Example: `bridge/account/gGhH....zZ4=/asset_id`.
///                         |base64 chars|
pub(in crate::bridge) fn asset_id<T: AddressBytes>(address: &T) -> String {
    format!(
        "{}/asset_id",
        AccountPrefixer::new(BRIDGE_ACCOUNT_PREFIX, address)
    )
}

/// Example: `bridge/sudo/gGhH....zZ4=`.
///                      |base64 chars|
pub(in crate::bridge) fn bridge_account_sudo_address<T: AddressBytes>(address: &T) -> String {
    AccountPrefixer::new(BRIDGE_ACCOUNT_SUDO_PREFIX, address).to_string()
}

/// Example: `bridge/withdrawer/gGhH....zZ4=`.
///                            |base64 chars|
pub(in crate::bridge) fn bridge_account_withdrawer_address<T: AddressBytes>(address: &T) -> String {
    AccountPrefixer::new(BRIDGE_ACCOUNT_WITHDRAWER_PREFIX, address).to_string()
}

/// Example: `bridge/account/gGhH....zZ4=/withdrawal_event/<event id>`.
///                         |base64 chars|                |UTF-8 chars|
pub(in crate::bridge) fn bridge_account_withdrawal_event<T: AddressBytes>(
    address: &T,
    withdrawal_event_id: &str,
) -> String {
    format!(
        "{}/withdrawal_event/{}",
        AccountPrefixer::new(BRIDGE_ACCOUNT_PREFIX, address),
        withdrawal_event_id
    )
}

pub(in crate::bridge) fn deposit(block_hash: &[u8; 32], rollup_id: &RollupId) -> String {
    format!(
        "{DEPOSIT_PREFIX}{}/{rollup_id}",
        Base64Display::new(block_hash, &URL_SAFE),
    )
}

pub(in crate::bridge) fn last_transaction_id_for_bridge_account<T: AddressBytes>(
    address: &T,
) -> String {
    format!(
        "{BRIDGE_ACCOUNT_PREFIX}{}/last_tx",
        Base64Display::new(address.address_bytes(), &URL_SAFE)
    )
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::Address;

    use super::*;

    const COMPONENT_PREFIX: &str = "bridge/";

    fn address() -> Address {
        "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
            .parse()
            .unwrap()
    }

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(INIT_BRIDGE_ACCOUNT_BASE_FEE);
        insta::assert_snapshot!(BRIDGE_LOCK_BYTE_COST_MULTIPLIER);
        insta::assert_snapshot!(BRIDGE_SUDO_CHANGE_FEE);
        insta::assert_snapshot!(DEPOSITS_EPHEMERAL);
        insta::assert_snapshot!(rollup_id(&address()));
        insta::assert_snapshot!(asset_id(&address()));
        insta::assert_snapshot!(bridge_account_sudo_address(&address()));
        insta::assert_snapshot!(bridge_account_withdrawer_address(&address()));
        insta::assert_snapshot!(bridge_account_withdrawal_event(&address(), "the-event"));
        insta::assert_snapshot!(deposit(&[1; 32], &RollupId::new([2; 32])));
        insta::assert_snapshot!(last_transaction_id_for_bridge_account(&address()));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(INIT_BRIDGE_ACCOUNT_BASE_FEE.starts_with(COMPONENT_PREFIX));
        assert!(BRIDGE_LOCK_BYTE_COST_MULTIPLIER.starts_with(COMPONENT_PREFIX));
        assert!(BRIDGE_SUDO_CHANGE_FEE.starts_with(COMPONENT_PREFIX));
        assert!(DEPOSITS_EPHEMERAL.starts_with(COMPONENT_PREFIX));
        assert!(rollup_id(&address()).starts_with(COMPONENT_PREFIX));
        assert!(asset_id(&address()).starts_with(COMPONENT_PREFIX));
        assert!(bridge_account_sudo_address(&address()).starts_with(COMPONENT_PREFIX));
        assert!(bridge_account_withdrawer_address(&address()).starts_with(COMPONENT_PREFIX));
        assert!(
            bridge_account_withdrawal_event(&address(), "the-event").starts_with(COMPONENT_PREFIX)
        );
        assert!(deposit(&[1; 32], &RollupId::new([2; 32])).starts_with(COMPONENT_PREFIX));
        assert!(last_transaction_id_for_bridge_account(&address()).starts_with(COMPONENT_PREFIX));
    }

    #[test]
    fn bridge_account_prefix_should_be_prefix_of_relevant_keys() {
        assert!(rollup_id(&address()).starts_with(BRIDGE_ACCOUNT_PREFIX));
        assert!(asset_id(&address()).starts_with(BRIDGE_ACCOUNT_PREFIX));
        assert!(
            bridge_account_withdrawal_event(&address(), "the-event")
                .starts_with(BRIDGE_ACCOUNT_PREFIX)
        );
        assert!(
            last_transaction_id_for_bridge_account(&address()).starts_with(BRIDGE_ACCOUNT_PREFIX)
        );
    }
}
