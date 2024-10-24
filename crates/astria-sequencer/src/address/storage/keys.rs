pub(in crate::address) const BASE_PREFIX: &str = "address/prefixes/base";
pub(in crate::address) const IBC_COMPAT_PREFIX: &str = "address/prefixes/ibc_compat";

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_PREFIX: &str = "address/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(BASE_PREFIX);
        insta::assert_snapshot!(IBC_COMPAT_PREFIX);
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(BASE_PREFIX.starts_with(COMPONENT_PREFIX));
        assert!(IBC_COMPAT_PREFIX.starts_with(COMPONENT_PREFIX));
    }
}
