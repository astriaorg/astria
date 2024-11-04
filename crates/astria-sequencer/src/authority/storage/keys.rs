pub(in crate::authority) const SUDO: &str = "authority/sudo";
pub(in crate::authority) const VALIDATOR_SET: &str = "authority/validator_set";
pub(in crate::authority) const VALIDATOR_UPDATES: &str = "authority/validator_updates";
pub(in crate::authority) const VALIDATOR_NAMES_PREFIX: &str = "authority/validator_names";

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_PREFIX: &str = "authority/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(SUDO);
        insta::assert_snapshot!(VALIDATOR_SET);
        insta::assert_snapshot!(VALIDATOR_UPDATES);
        insta::assert_snapshot!(VALIDATOR_NAMES_PREFIX);
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(SUDO.starts_with(COMPONENT_PREFIX));
        assert!(VALIDATOR_SET.starts_with(COMPONENT_PREFIX));
        assert!(VALIDATOR_UPDATES.starts_with(COMPONENT_PREFIX));
        assert!(VALIDATOR_NAMES_PREFIX.starts_with(COMPONENT_PREFIX));
    }
}
