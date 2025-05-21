use astria_core::{
    generated::astria::primitive::v1::Uint128,
    protocol::orderbook::v1 as proto,
};

use crate::orderbook::component::{OrderSide, OrderType, OrderTimeInForce};

/// Convert a protocol OrderSide to our local OrderSide
pub fn order_side_from_proto(side: proto::OrderSide) -> OrderSide {
    match side {
        proto::OrderSide::Buy => OrderSide::Buy,
        proto::OrderSide::Sell => OrderSide::Sell,
        _ => OrderSide::Buy, // Default to Buy for unspecified/unknown
    }
}

/// Convert an i32 representing OrderSide to the enum
pub fn order_side_from_i32(side: i32) -> proto::OrderSide {
    let result = proto::OrderSide::try_from(side);
    
    // Add detailed logging for debugging SELL order issues
    match result {
        Ok(proto::OrderSide::Sell) => {
            tracing::warn!(" utils::order_side_from_i32 parsed value {} as SELL", side);
        },
        Ok(proto::OrderSide::Buy) => {
            tracing::warn!(" utils::order_side_from_i32 parsed value {} as BUY", side);
        },
        Ok(proto::OrderSide::Unspecified) => {
            tracing::error!(" utils::order_side_from_i32 parsed value {} as UNSPECIFIED", side);
        },
        Err(err) => {
            tracing::error!(" utils::order_side_from_i32 failed to parse value {}: {}", side, err);
        }
    }
    
    result.unwrap_or(proto::OrderSide::Unspecified)
}

/// Convert a protocol OrderType to our local OrderType
pub fn order_type_from_proto(order_type: proto::OrderType) -> OrderType {
    match order_type {
        proto::OrderType::Limit => OrderType::Limit,
        proto::OrderType::Market => OrderType::Market,
        _ => OrderType::Limit, // Default to Limit for unspecified/unknown
    }
}

/// Convert an i32 representing OrderType to the enum
pub fn order_type_from_i32(order_type: i32) -> proto::OrderType {
    proto::OrderType::try_from(order_type).unwrap_or(proto::OrderType::Unspecified)
}

/// Convert a protocol OrderTimeInForce to our local OrderTimeInForce
pub fn time_in_force_from_proto(time_in_force: proto::OrderTimeInForce) -> OrderTimeInForce {
    match time_in_force {
        proto::OrderTimeInForce::Gtc => OrderTimeInForce::GoodTillCancelled,
        proto::OrderTimeInForce::Fok => OrderTimeInForce::FillOrKill,
        proto::OrderTimeInForce::Ioc => OrderTimeInForce::ImmediateOrCancel,
        _ => OrderTimeInForce::GoodTillCancelled, // Default for unspecified/unknown
    }
}

/// Convert an i32 representing OrderTimeInForce to the protocol enum
pub fn time_in_force_from_i32(time_in_force: i32) -> proto::OrderTimeInForce {
    proto::OrderTimeInForce::try_from(time_in_force).unwrap_or(proto::OrderTimeInForce::Unspecified)
}

/// Convert an Option<Uint128> to a string representation
pub fn uint128_option_to_string(value: &Option<Uint128>) -> String {
    match value {
        Some(uint128) => {
            // Reconstruct u128 from hi and lo (as described in the Uint128 docs)
            let val = ((uint128.hi as u128) << 64) + (uint128.lo as u128);
            val.to_string()
        }
        None => "0".to_string(),
    }
}

/// Convert an Option<primitive::v1::Uint128> to a string representation
pub fn primitive_uint128_option_to_string(value: &Option<astria_core::generated::astria::primitive::v1::Uint128>) -> String {
    match value {
        Some(uint128) => {
            // Reconstruct u128 from hi and lo (as described in the Uint128 docs)
            let val = ((uint128.hi as u128) << 64) + (uint128.lo as u128);
            val.to_string()
        }
        None => "0".to_string(),
    }
}

/// Parse a string to u128
pub fn parse_string_to_u128(s: &str) -> u128 {
    s.parse::<u128>().unwrap_or(0)
}

/// Convert a string to Option<Uint128>
pub fn string_to_uint128_option(s: &str) -> Option<Uint128> {
    match s.parse::<u128>() {
        Ok(val) => {
            let hi = (val >> 64) as u64;
            let lo = val as u64;
            Some(Uint128 { hi, lo })
        }
        Err(_) => None,
    }
}

/// Format an Option<Uint128> for display purposes
pub fn format_uint128_option(value: &Option<Uint128>) -> String {
    uint128_option_to_string(value)
}