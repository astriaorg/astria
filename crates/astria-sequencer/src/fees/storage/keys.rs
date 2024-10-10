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
pub(in crate::fees) const BLOCK: &str = "fees/block_fees";

#[cfg(test)]
mod tests {
    use std::fmt::{
        self,
        Display,
        Formatter,
    };

    use super::*;

    const COMPONENT_PREFIX: &str = "fees/";

    #[test]
    fn keys_should_not_change() {
        // NOTE: This helper struct is just to avoid having 14 snapshot files to contend with.
        // NOTE: `BLOCK` is only used in the ephemeral store, so isn't included here.
        struct Helper;
        impl Display for Helper {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                writeln!(formatter, "{TRANSFER}")?;
                writeln!(formatter, "{SEQUENCE}")?;
                writeln!(formatter, "{ICS20_WITHDRAWAL}")?;
                writeln!(formatter, "{INIT_BRIDGE_ACCOUNT}")?;
                writeln!(formatter, "{BRIDGE_LOCK}")?;
                writeln!(formatter, "{BRIDGE_UNLOCK}")?;
                writeln!(formatter, "{BRIDGE_SUDO_CHANGE}")?;
                writeln!(formatter, "{IBC_RELAY}")?;
                writeln!(formatter, "{VALIDATOR_UPDATE}")?;
                writeln!(formatter, "{FEE_ASSET_CHANGE}")?;
                writeln!(formatter, "{FEE_CHANGE}")?;
                writeln!(formatter, "{IBC_RELAYER_CHANGE}")?;
                writeln!(formatter, "{SUDO_ADDRESS_CHANGE}")?;
                writeln!(formatter, "{IBC_SUDO_CHANGE}")?;
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
    }
}
