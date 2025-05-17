use astria_core::{
    generated::astria::primitive::v1::Uint128,
    protocol::orderbook::v1::{OrderSide, OrderType, OrderTimeInForce},
};

/// Convert an i32 representing OrderSide to the enum
pub fn order_side_from_i32(side: i32) -> OrderSide {
    OrderSide::try_from(side).unwrap_or(OrderSide::Unspecified)
}

/// Convert an i32 representing OrderType to the enum
pub fn order_type_from_i32(order_type: i32) -> OrderType {
    OrderType::try_from(order_type).unwrap_or(OrderType::Unspecified)
}

/// Convert an i32 representing OrderTimeInForce to the enum
pub fn time_in_force_from_i32(time_in_force: i32) -> OrderTimeInForce {
    OrderTimeInForce::try_from(time_in_force).unwrap_or(OrderTimeInForce::Unspecified)
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