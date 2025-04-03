use std::borrow::Cow;

use astria_core::oracles::price_feed::types::v2::{
    Base,
    CurrencyPair as DomainCurrencyPair,
    Quote,
};
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
pub(in crate::oracles::price_feed::oracle) struct CurrencyPair<'a> {
    base: Cow<'a, str>,
    quote: Cow<'a, str>,
}

impl<'a> From<&'a DomainCurrencyPair> for CurrencyPair<'a> {
    fn from(currency_pair: &'a DomainCurrencyPair) -> Self {
        CurrencyPair {
            base: Cow::Borrowed(currency_pair.base()),
            quote: Cow::Borrowed(currency_pair.quote()),
        }
    }
}

impl<'a> From<CurrencyPair<'a>> for DomainCurrencyPair {
    fn from(currency_pair: CurrencyPair<'a>) -> Self {
        DomainCurrencyPair::from_parts(
            Base::unchecked_from_parts(currency_pair.base.into_owned()),
            Quote::unchecked_from_parts(currency_pair.quote.into_owned()),
        )
    }
}

impl<'a> From<CurrencyPair<'a>> for crate::storage::StoredValue<'a> {
    fn from(currency_pair: CurrencyPair<'a>) -> Self {
        crate::storage::StoredValue::PriceFeedOracle(Value(ValueImpl::CurrencyPair(currency_pair)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for CurrencyPair<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::PriceFeedOracle(Value(ValueImpl::CurrencyPair(
            currency_pair,
        ))) = value
        else {
            bail!(
                "price feed oracle stored value type mismatch: expected currency pair, found \
                 {value:?}"
            );
        };
        Ok(currency_pair)
    }
}
