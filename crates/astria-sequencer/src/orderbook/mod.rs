pub mod component;
pub mod compat;
pub mod debug;
mod matching_engine;
pub mod mock_data;
pub mod query;
pub mod state_ext;
pub mod types;
pub mod utils;
#[cfg(test)]
mod tests;

pub use component::OrderbookComponent;
pub use state_ext::{StateReadExt, StateWriteExt};
pub use types::{Market, Order, OrderbookEntry, Trade, OrderbookDepth, OrderbookDepthLevel};
pub use utils::{
    order_side_from_i32, parse_string_to_u128,
    string_to_uint128_option, uint128_option_to_string,
};