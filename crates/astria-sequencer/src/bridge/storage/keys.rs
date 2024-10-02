use astria_core::primitive::v1::RollupId;

use crate::{
    accounts::AddressBytes,
    storage::keys::AddressPrefixer,
};

pub(in crate::bridge) const INIT_BRIDGE_ACCOUNT_BASE_FEE_KEY: &str = "bridge/init_account_fee";
pub(in crate::bridge) const BRIDGE_LOCK_BYTE_COST_MULTIPLIER_KEY: &str =
    "bridge/lock_byte_cost_multiplier";
pub(in crate::bridge) const BRIDGE_SUDO_CHANGE_FEE_KEY: &str = "bridge/sudo_change_fee";

pub(in crate::bridge) const BRIDGE_ACCOUNT_PREFIX: &str = "bridge/account/";
const BRIDGE_ACCOUNT_SUDO_PREFIX: &str = "bridge/sudo/";
const BRIDGE_ACCOUNT_WITHDRAWER_PREFIX: &str = "bridge/withdrawer/";

pub(in crate::bridge) const DEPOSITS_EPHEMERAL_KEY: &str = "bridge/deposits";
const DEPOSIT_PREFIX: &[u8] = b"bridge/deposit/";

/// Example: `bridge/account/0101....0101/rollup_id`.
///                         |40 hex chars|
pub(in crate::bridge) fn rollup_id_key<T: AddressBytes>(address: &T) -> String {
    format!(
        "{}/rollup_id",
        AddressPrefixer::new(BRIDGE_ACCOUNT_PREFIX, address)
    )
}

/// Example: `bridge/account/0101....0101/asset_id`.
///                         |40 hex chars|
pub(in crate::bridge) fn asset_id_key<T: AddressBytes>(address: &T) -> String {
    format!(
        "{}/asset_id",
        AddressPrefixer::new(BRIDGE_ACCOUNT_PREFIX, address)
    )
}

/// Example: `bridge/sudo/0101....0101`.
///                      |40 hex chars|
pub(in crate::bridge) fn bridge_account_sudo_address_key<T: AddressBytes>(address: &T) -> String {
    AddressPrefixer::new(BRIDGE_ACCOUNT_SUDO_PREFIX, address).to_string()
}

/// Example: `bridge/withdrawer/0101....0101`.
///                            |40 hex chars|
pub(in crate::bridge) fn bridge_account_withdrawer_address_key<T: AddressBytes>(
    address: &T,
) -> String {
    AddressPrefixer::new(BRIDGE_ACCOUNT_WITHDRAWER_PREFIX, address).to_string()
}

/// Example: `bridge/account/0101....0101/withdrawal_event/<event id>`.
///                         |40 hex chars|                |UTF-8 chars|
pub(in crate::bridge) fn bridge_account_withdrawal_event_key<T: AddressBytes>(
    address: &T,
    withdrawal_event_id: &str,
) -> String {
    format!(
        "{}/withdrawal_event/{}",
        AddressPrefixer::new(BRIDGE_ACCOUNT_PREFIX, address),
        withdrawal_event_id
    )
}

pub(in crate::bridge) fn deposit_key(block_hash: &[u8; 32], rollup_id: &RollupId) -> Vec<u8> {
    [DEPOSIT_PREFIX, block_hash, rollup_id.as_ref()].concat()
}

pub(in crate::bridge) fn last_transaction_id_for_bridge_account_key<T: AddressBytes>(
    address: &T,
) -> Vec<u8> {
    [
        BRIDGE_ACCOUNT_PREFIX.as_bytes(),
        address.address_bytes(),
        b"/last_tx",
    ]
    .concat()
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::Address;
    use telemetry::display::base64;

    use super::*;

    const COMPONENT_PREFIX: &str = "bridge/";

    fn address() -> Address {
        "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
            .parse()
            .unwrap()
    }

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(INIT_BRIDGE_ACCOUNT_BASE_FEE_KEY);
        insta::assert_snapshot!(BRIDGE_LOCK_BYTE_COST_MULTIPLIER_KEY);
        insta::assert_snapshot!(BRIDGE_SUDO_CHANGE_FEE_KEY);
        insta::assert_snapshot!(DEPOSITS_EPHEMERAL_KEY);
        insta::assert_snapshot!(rollup_id_key(&address()));
        insta::assert_snapshot!(asset_id_key(&address()));
        insta::assert_snapshot!(bridge_account_sudo_address_key(&address()));
        insta::assert_snapshot!(bridge_account_withdrawer_address_key(&address()));
        insta::assert_snapshot!(bridge_account_withdrawal_event_key(&address(), "the-event"));
        insta::assert_snapshot!(base64(&deposit_key(&[1; 32], &RollupId::new([2; 32]))));
        insta::assert_snapshot!(base64(&last_transaction_id_for_bridge_account_key(
            &address()
        )));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(INIT_BRIDGE_ACCOUNT_BASE_FEE_KEY.starts_with(COMPONENT_PREFIX));
        assert!(BRIDGE_LOCK_BYTE_COST_MULTIPLIER_KEY.starts_with(COMPONENT_PREFIX));
        assert!(BRIDGE_SUDO_CHANGE_FEE_KEY.starts_with(COMPONENT_PREFIX));
        assert!(DEPOSITS_EPHEMERAL_KEY.starts_with(COMPONENT_PREFIX));
        assert!(rollup_id_key(&address()).starts_with(COMPONENT_PREFIX));
        assert!(asset_id_key(&address()).starts_with(COMPONENT_PREFIX));
        assert!(bridge_account_sudo_address_key(&address()).starts_with(COMPONENT_PREFIX));
        assert!(bridge_account_withdrawer_address_key(&address()).starts_with(COMPONENT_PREFIX));
        assert!(
            bridge_account_withdrawal_event_key(&address(), "the-event")
                .starts_with(COMPONENT_PREFIX)
        );
        assert!(
            deposit_key(&[1; 32], &RollupId::new([2; 32])).starts_with(COMPONENT_PREFIX.as_bytes())
        );
        assert!(
            last_transaction_id_for_bridge_account_key(&address())
                .starts_with(COMPONENT_PREFIX.as_bytes())
        );
    }

    #[test]
    fn bridge_account_prefix_should_be_prefix_of_relevant_keys() {
        assert!(rollup_id_key(&address()).starts_with(BRIDGE_ACCOUNT_PREFIX));
        assert!(asset_id_key(&address()).starts_with(BRIDGE_ACCOUNT_PREFIX));
        assert!(
            bridge_account_withdrawal_event_key(&address(), "the-event")
                .starts_with(BRIDGE_ACCOUNT_PREFIX)
        );
        assert!(
            last_transaction_id_for_bridge_account_key(&address())
                .starts_with(BRIDGE_ACCOUNT_PREFIX.as_bytes())
        );
    }
}
