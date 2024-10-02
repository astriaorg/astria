pub(in crate::authority) const SUDO_KEY: &str = "authority/sudo";
pub(in crate::authority) const VALIDATOR_SET_KEY: &str = "authority/validator_set";
pub(in crate::authority) const VALIDATOR_UPDATES_KEY: &[u8] = b"authority/validator_updates";

#[cfg(test)]
mod tests {
    use telemetry::display::base64;

    use super::*;

    const COMPONENT_PREFIX: &str = "authority/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(SUDO_KEY);
        insta::assert_snapshot!(VALIDATOR_SET_KEY);
        insta::assert_snapshot!(base64(VALIDATOR_UPDATES_KEY));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(SUDO_KEY.starts_with(COMPONENT_PREFIX));
        assert!(VALIDATOR_SET_KEY.starts_with(COMPONENT_PREFIX));
        assert!(VALIDATOR_UPDATES_KEY.starts_with(COMPONENT_PREFIX.as_bytes()));
    }
}
