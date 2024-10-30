pub(in crate::connect::marketmap) const MARKET_MAP: &str = "connect/market_map/market_map";
pub(in crate::connect::marketmap) const PARAMS: &str = "connect/market_map/params";
pub(in crate::connect::marketmap) const LAST_UPDATED: &str = "connect/market_map/last_updated";

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_PREFIX: &str = "connect/market_map/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!("market_map_key", MARKET_MAP);
        insta::assert_snapshot!("params_key", PARAMS);
        insta::assert_snapshot!("last_updated_key", LAST_UPDATED);
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(MARKET_MAP.starts_with(COMPONENT_PREFIX));
        assert!(PARAMS.starts_with(COMPONENT_PREFIX));
        assert!(LAST_UPDATED.starts_with(COMPONENT_PREFIX));
    }
}
