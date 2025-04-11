pub(in crate::oracles::price_feed::market_map) const MARKET_MAP: &str =
    "price_feed/market_map/market_map";
pub(in crate::oracles::price_feed::market_map) const LAST_UPDATED: &str =
    "price_feed/market_map/last_updated";

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_PREFIX: &str = "price_feed/market_map/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!("market_map_key", MARKET_MAP);
        insta::assert_snapshot!("last_updated_key", LAST_UPDATED);
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(MARKET_MAP.starts_with(COMPONENT_PREFIX));
        assert!(LAST_UPDATED.starts_with(COMPONENT_PREFIX));
    }
}
