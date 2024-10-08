pub(in crate::sequence) const SEQUENCE_ACTION_BASE_FEE: &str = "sequence/base_fee";
pub(in crate::sequence) const SEQUENCE_ACTION_BYTE_COST_MULTIPLIER: &str =
    "sequence/byte_cost_multiplier";

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_PREFIX: &str = "sequence/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(SEQUENCE_ACTION_BASE_FEE);
        insta::assert_snapshot!(SEQUENCE_ACTION_BYTE_COST_MULTIPLIER);
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(SEQUENCE_ACTION_BASE_FEE.starts_with(COMPONENT_PREFIX));
        assert!(SEQUENCE_ACTION_BYTE_COST_MULTIPLIER.starts_with(COMPONENT_PREFIX));
    }
}
