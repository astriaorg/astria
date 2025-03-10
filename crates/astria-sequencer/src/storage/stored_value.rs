use astria_eyre::{
    eyre::WrapErr as _,
    Result,
};
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) enum StoredValue<'a> {
    Unit,
    Address(crate::address::storage::Value<'a>),
    Assets(crate::assets::storage::Value<'a>),
    Accounts(crate::accounts::storage::Value),
    Authority(crate::authority::storage::Value<'a>),
    Fees(crate::fees::storage::Value),
    Bridge(crate::bridge::storage::Value<'a>),
    Ibc(crate::ibc::storage::Value<'a>),
    App(crate::app::storage::Value<'a>),
    Grpc(crate::grpc::storage::Value<'a>),
    Upgrades(crate::upgrades::storage::Value<'a>),
    PriceFeedMarketMap(crate::oracles::price_feed::market_map::storage::Value<'a>),
    PriceFeedOracle(crate::oracles::price_feed::oracle::storage::Value<'a>),
}

impl StoredValue<'_> {
    pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
        borsh::to_vec(&self).wrap_err("failed to serialize stored value")
    }

    pub(crate) fn deserialize(bytes: &[u8]) -> Result<Self> {
        borsh::from_slice(bytes).wrap_err("failed to deserialize stored value")
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;
    use crate::test_utils::borsh_then_hex;

    #[test]
    fn stored_value_unit_variant_unchanged() {
        assert_snapshot!(
            "stored_value_unit_variant",
            borsh_then_hex(&StoredValue::Unit)
        );
    }
}
