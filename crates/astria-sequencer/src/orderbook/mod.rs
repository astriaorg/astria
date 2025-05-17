pub mod component;
pub mod compat;
mod matching_engine;
mod query;
pub mod state_ext;
mod types;
mod utils;
#[cfg(test)]
mod tests;

pub use component::OrderbookComponent;
use matching_engine::MatchingEngine;
pub use state_ext::{MarketParams, OrderbookError, StateReadExt, StateWriteExt};
pub use types::{
    CancelOrderAction, CreateMarketAction, CreateOrderAction, Market, Order, OrderMatch, OrderSide, 
    OrderTimeInForce, OrderType, Orderbook, OrderbookEntry, Trade, UpdateMarketAction, opposing_side,
};
pub use utils::{
    order_side_from_i32, order_type_from_i32, time_in_force_from_i32,
    uint128_option_to_string, parse_string_to_u128, string_to_uint128_option,
    format_uint128_option, primitive_uint128_option_to_string,
};