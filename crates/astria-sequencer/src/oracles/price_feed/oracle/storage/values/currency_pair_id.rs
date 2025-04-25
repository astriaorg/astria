use astria_core::oracles::price_feed::types::v2::CurrencyPairId as DomainCurrencyPairId;
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::oracles::price_feed::oracle) struct CurrencyPairId(u64);

impl From<DomainCurrencyPairId> for CurrencyPairId {
    fn from(id: DomainCurrencyPairId) -> Self {
        CurrencyPairId(id.get())
    }
}

impl From<CurrencyPairId> for DomainCurrencyPairId {
    fn from(id: CurrencyPairId) -> Self {
        DomainCurrencyPairId::new(id.0)
    }
}

impl From<CurrencyPairId> for crate::storage::StoredValue<'_> {
    fn from(id: CurrencyPairId) -> Self {
        crate::storage::StoredValue::PriceFeedOracle(Value(ValueImpl::CurrencyPairId(id)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for CurrencyPairId {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::PriceFeedOracle(Value(ValueImpl::CurrencyPairId(id))) =
            value
        else {
            bail!(
                "price feed oracle stored value type mismatch: expected currency pair id, found \
                 {value:?}"
            );
        };
        Ok(id)
    }
}
