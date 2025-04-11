use crate::accounts::AddressBytes;

pub(in crate::authority) const SUDO: &str = "authority/sudo";
pub(in crate::authority) const VALIDATOR_PREFIX: &str = "authority/validator/";
pub(in crate::authority) const VALIDATOR_COUNT: &str = "authority/validator_count";
pub(in crate::authority) const VALIDATOR_UPDATES: &str = "authority/validator_updates";

pub(in crate::authority) const PRE_ASPEN_VALIDATOR_SET: &str = "authority/validator_set"; // Deprecated post Aspen upgrade

pub(in crate::authority) fn validator<TAddress: AddressBytes>(address: &TAddress) -> String {
    format!(
        "{}{}",
        VALIDATOR_PREFIX,
        hex::encode(address.address_bytes())
    )
}

#[cfg(test)]
mod tests {
    use astria_core::{
        crypto::ADDRESS_LENGTH,
        primitive::v1::Address,
    };

    use super::*;

    const COMPONENT_PREFIX: &str = "authority/";

    fn snapshot_validator() -> String {
        validator(
            &Address::builder()
                .array([1; ADDRESS_LENGTH])
                .prefix("astria")
                .try_build()
                .unwrap(),
        )
    }

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!("sudo_address_key", SUDO);
        insta::assert_snapshot!("validator_key", snapshot_validator());
        insta::assert_snapshot!("validator_count_key", VALIDATOR_COUNT);

        insta::assert_snapshot!("validator_set_key", PRE_ASPEN_VALIDATOR_SET);
        insta::assert_snapshot!("validator_updates_key", VALIDATOR_UPDATES);
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(SUDO.starts_with(COMPONENT_PREFIX));
        assert!(snapshot_validator().starts_with(COMPONENT_PREFIX));
        assert!(VALIDATOR_COUNT.starts_with(COMPONENT_PREFIX));

        assert!(PRE_ASPEN_VALIDATOR_SET.starts_with(COMPONENT_PREFIX));
        assert!(VALIDATOR_UPDATES.starts_with(COMPONENT_PREFIX));
    }
}
