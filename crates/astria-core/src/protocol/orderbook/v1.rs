//! Order book protocol types for the Astria network.
//!
//! This module re-exports the auto-generated protobuf types from the Astria protocol
//! for the orderbook functionality.

// Re-export generated types
pub use crate::generated::astria::protocol::orderbook::v1::{
    Order, OrderMatch, Orderbook, OrderbookEntry, OrderSide, OrderTimeInForce, OrderType,
};
