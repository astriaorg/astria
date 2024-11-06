use astria_core::{
    connect::{
        oracle::v2::{
            CurrencyPairState as DomainCurrencyPairState,
            QuotePrice as DomainQuotePrice,
        },
        types::v2::{
            CurrencyPairId as DomainCurrencyPairId,
            CurrencyPairNonce,
            Price,
        },
    },
    primitive::Timestamp as DomainTimestamp,
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    CurrencyPairId,
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct Timestamp {
    seconds: i64,
    nanos: i32,
}

impl From<DomainTimestamp> for Timestamp {
    fn from(timestamp: DomainTimestamp) -> Self {
        Self {
            seconds: timestamp.seconds,
            nanos: timestamp.nanos,
        }
    }
}

impl From<Timestamp> for DomainTimestamp {
    fn from(timestamp: Timestamp) -> Self {
        Self {
            seconds: timestamp.seconds,
            nanos: timestamp.nanos,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct QuotePrice {
    price: u128,
    block_timestamp: Timestamp,
    block_height: u64,
}

impl From<DomainQuotePrice> for QuotePrice {
    fn from(quote_price: DomainQuotePrice) -> Self {
        Self {
            price: quote_price.price.get(),
            block_timestamp: Timestamp::from(quote_price.block_timestamp),
            block_height: quote_price.block_height,
        }
    }
}

impl From<QuotePrice> for DomainQuotePrice {
    fn from(quote_price: QuotePrice) -> Self {
        Self {
            price: Price::new(quote_price.price),
            block_timestamp: DomainTimestamp::from(quote_price.block_timestamp),
            block_height: quote_price.block_height,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::connect::oracle) struct CurrencyPairState {
    price: QuotePrice,
    nonce: u64,
    id: CurrencyPairId,
}

impl From<DomainCurrencyPairState> for CurrencyPairState {
    fn from(state: DomainCurrencyPairState) -> Self {
        CurrencyPairState {
            price: QuotePrice::from(state.price),
            nonce: state.nonce.get(),
            id: CurrencyPairId::from(state.id),
        }
    }
}

impl From<CurrencyPairState> for DomainCurrencyPairState {
    fn from(state: CurrencyPairState) -> Self {
        Self {
            price: DomainQuotePrice::from(state.price),
            nonce: CurrencyPairNonce::new(state.nonce),
            id: DomainCurrencyPairId::from(state.id),
        }
    }
}

impl<'a> From<CurrencyPairState> for crate::storage::StoredValue<'a> {
    fn from(state: CurrencyPairState) -> Self {
        crate::storage::StoredValue::ConnectOracle(Value(ValueImpl::CurrencyPairState(state)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for CurrencyPairState {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::ConnectOracle(Value(ValueImpl::CurrencyPairState(state))) =
            value
        else {
            bail!(
                "connect oracle stored value type mismatch: expected currency pair state, found \
                 {value:?}"
            );
        };
        Ok(state)
    }
}
