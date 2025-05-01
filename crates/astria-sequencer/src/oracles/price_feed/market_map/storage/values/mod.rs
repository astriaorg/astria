use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

pub(in crate::oracles::price_feed::market_map) use self::{
    block_height::BlockHeight,
    market_map::MarketMap,
};

mod block_height;
mod market_map;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    MarketMap(MarketMap<'a>),
    BlockHeight(BlockHeight),
}
