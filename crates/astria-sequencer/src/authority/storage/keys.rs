pub(in crate::authority) const SUDO: &str = "authority/sudo";
pub(in crate::authority) const VALIDATOR_PREFIX: &str = "authority/validator/";
pub(in crate::authority) const VALIDATOR_COUNT: &str = "authority/validator_count";
pub(in crate::authority) const VALIDATOR_UPDATES: &str = "authority/validator_updates";

pub(in crate::authority) const _PRE_ASPEN_VALIDATOR_SET: &str = "authority/validator_set"; // Deprecated post Aspen upgrade

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_PREFIX: &str = "authority/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!("sudo_address_key", SUDO);
        insta::assert_snapshot!("validator_prefix", VALIDATOR_PREFIX);
        insta::assert_snapshot!("validator_count_key", VALIDATOR_COUNT);

        insta::assert_snapshot!("validator_set_key", _PRE_ASPEN_VALIDATOR_SET);
        insta::assert_snapshot!("validator_updates_key", VALIDATOR_UPDATES);
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(SUDO.starts_with(COMPONENT_PREFIX));
        assert!(VALIDATOR_PREFIX.starts_with(COMPONENT_PREFIX));
        assert!(VALIDATOR_COUNT.starts_with(COMPONENT_PREFIX));

        assert!(_PRE_ASPEN_VALIDATOR_SET.starts_with(COMPONENT_PREFIX));
        assert!(VALIDATOR_UPDATES.starts_with(COMPONENT_PREFIX));
    }
}
