pub(in crate::address) const BASE_PREFIX_KEY: &str = "address/prefixes/base";
pub(in crate::address) const IBC_COMPAT_PREFIX_KEY: &str = "address/prefixes/ibc_compat";

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_PREFIX: &str = "address/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(BASE_PREFIX_KEY);
        insta::assert_snapshot!(IBC_COMPAT_PREFIX_KEY);
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(BASE_PREFIX_KEY.starts_with(COMPONENT_PREFIX));
        assert!(IBC_COMPAT_PREFIX_KEY.starts_with(COMPONENT_PREFIX));
    }
}
