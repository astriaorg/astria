pub mod abci;
pub mod market_map;
pub mod oracle;
pub mod service;
pub mod types;
pub mod utils;

#[cfg(test)]
mod tests {
    use crate::protocol::test_utils::dummy_price_feed_genesis;

    #[test]
    fn serialized_market_map_and_oracle_should_not_change() {
        let serialized_price_feed_genesis =
            hex::encode(borsh::to_vec(&dummy_price_feed_genesis()).unwrap());
        insta::assert_snapshot!("price_feed_genesis", serialized_price_feed_genesis);
    }
}
