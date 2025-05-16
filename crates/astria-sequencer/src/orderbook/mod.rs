mod component;
mod compat;
mod matching_engine;
mod query;
mod state_ext;
mod types;
#[cfg(test)]
mod tests;

pub use component::OrderbookComponent;
use matching_engine::MatchingEngine;
pub use state_ext::{MarketParams, OrderbookError, StateReadExt, StateWriteExt};
pub use types::{
    CancelOrderAction, CreateMarketAction, CreateOrderAction, Market, Order, OrderMatch, OrderSide, 
    OrderTimeInForce, OrderType, Orderbook, OrderbookEntry, Trade, UpdateMarketAction, opposing_side,
};