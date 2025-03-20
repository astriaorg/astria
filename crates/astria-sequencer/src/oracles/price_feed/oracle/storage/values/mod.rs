use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

pub(in crate::oracles::price_feed::oracle) use self::{
    count::Count,
    currency_pair::CurrencyPair,
    currency_pair_id::CurrencyPairId,
    currency_pair_state::CurrencyPairState,
};

mod count;
mod currency_pair;
mod currency_pair_id;
mod currency_pair_state;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    CurrencyPairId(CurrencyPairId),
    CurrencyPair(CurrencyPair<'a>),
    Count(Count),
    CurrencyPairState(CurrencyPairState),
}
