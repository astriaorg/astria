use astria_core::protocol::orderbook::v1 as proto;
use borsh::{BorshSerialize, BorshDeserialize};
use prost::Message;

use crate::orderbook::types::{
    Order, OrderSide, OrderTimeInForce, OrderType, Market as LocalMarket, 
    OrderMatch as LocalOrderMatch, Trade as LocalTrade
};
use crate::orderbook::state_ext::MarketParams;

// Implement Borsh serialization for proto::Order
impl BorshSerialize for proto::Order {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.encode_to_vec();
        BorshSerialize::serialize(&bytes, writer)
    }
}

// Implement Borsh deserialization for proto::Order
impl BorshDeserialize for proto::Order {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes: Vec<u8> = BorshDeserialize::deserialize_reader(reader)?;
        proto::Order::decode(&*bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

// Implement Borsh serialization for proto::OrderMatch
impl BorshSerialize for proto::OrderMatch {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.encode_to_vec();
        BorshSerialize::serialize(&bytes, writer)
    }
}

// Implement Borsh deserialization for proto::OrderMatch
impl BorshDeserialize for proto::OrderMatch {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes: Vec<u8> = BorshDeserialize::deserialize_reader(reader)?;
        proto::OrderMatch::decode(&*bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

// Implement Borsh serialization for proto::OrderbookEntry
impl BorshSerialize for proto::OrderbookEntry {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.encode_to_vec();
        BorshSerialize::serialize(&bytes, writer)
    }
}

// Implement Borsh deserialization for proto::OrderbookEntry
impl BorshDeserialize for proto::OrderbookEntry {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes: Vec<u8> = BorshDeserialize::deserialize_reader(reader)?;
        proto::OrderbookEntry::decode(&*bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

// Implement Borsh serialization for proto::Orderbook
impl BorshSerialize for proto::Orderbook {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.encode_to_vec();
        BorshSerialize::serialize(&bytes, writer)
    }
}

// Implement Borsh deserialization for proto::Orderbook
impl BorshDeserialize for proto::Orderbook {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes: Vec<u8> = BorshDeserialize::deserialize_reader(reader)?;
        proto::Orderbook::decode(&*bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Converts from protocol OrderSide to our local OrderSide
pub fn order_side_from_proto(side: proto::OrderSide) -> OrderSide {
    match side {
        proto::OrderSide::Buy => OrderSide::Buy,
        proto::OrderSide::Sell => OrderSide::Sell,
        _ => OrderSide::Buy, // Default to Buy for unspecified
    }
}

/// Converts from our local OrderSide to protocol OrderSide
pub fn order_side_to_proto(side: OrderSide) -> proto::OrderSide {
    match side {
        OrderSide::Buy => proto::OrderSide::Buy,
        OrderSide::Sell => proto::OrderSide::Sell,
    }
}

/// Converts from protocol OrderType to our local OrderType
pub fn order_type_from_proto(order_type: proto::OrderType) -> OrderType {
    match order_type {
        proto::OrderType::Limit => OrderType::Limit,
        proto::OrderType::Market => OrderType::Market,
        _ => OrderType::Limit, // Default to Limit for unspecified
    }
}

/// Converts from our local OrderType to protocol OrderType
pub fn order_type_to_proto(order_type: OrderType) -> proto::OrderType {
    match order_type {
        OrderType::Limit => proto::OrderType::Limit,
        OrderType::Market => proto::OrderType::Market,
    }
}

/// Converts from protocol OrderTimeInForce to our local OrderTimeInForce
pub fn time_in_force_from_proto(time_in_force: proto::OrderTimeInForce) -> OrderTimeInForce {
    match time_in_force {
        proto::OrderTimeInForce::Gtc => OrderTimeInForce::GoodTillCancelled,
        proto::OrderTimeInForce::Ioc => OrderTimeInForce::ImmediateOrCancel,
        proto::OrderTimeInForce::Fok => OrderTimeInForce::FillOrKill,
        _ => OrderTimeInForce::GoodTillCancelled, // Default to GTC for unspecified
    }
}

/// Converts from our local OrderTimeInForce to protocol OrderTimeInForce
pub fn time_in_force_to_proto(time_in_force: OrderTimeInForce) -> proto::OrderTimeInForce {
    match time_in_force {
        OrderTimeInForce::GoodTillCancelled => proto::OrderTimeInForce::Gtc,
        OrderTimeInForce::ImmediateOrCancel => proto::OrderTimeInForce::Ioc,
        OrderTimeInForce::FillOrKill => proto::OrderTimeInForce::Fok,
    }
}

/// Converts from protocol Order to our local Order
pub fn order_from_proto(proto_order: &proto::Order) -> Order {
    Order {
        id: proto_order.id.clone(),
        owner: match &proto_order.owner {
            Some(addr) => addr.bech32m.clone(),
            None => "".to_string(),
        },
        market: proto_order.market.clone(),
        side: order_side_from_proto(proto_order.side()),
        order_type: order_type_from_proto(proto_order.r#type()),
        price: crate::orderbook::uint128_option_to_string(&proto_order.price),
        quantity: crate::orderbook::uint128_option_to_string(&proto_order.quantity),
        remaining_quantity: crate::orderbook::uint128_option_to_string(&proto_order.remaining_quantity),
        created_at: proto_order.created_at,
        time_in_force: time_in_force_from_proto(proto_order.time_in_force()),
    }
}

/// Converts from our local Order to protocol Order
pub fn order_to_proto(order: &Order) -> proto::Order {
    // Create an Option<Address> from the owner string
    let owner = if order.owner.is_empty() {
        None
    } else {
        Some(astria_core::primitive::v1::Address {
            bech32m: order.owner.clone(),
        })
    };
    
    proto::Order {
        id: order.id.clone(),
        owner,
        market: order.market.clone(),
        side: order_side_to_proto(order.side) as i32,
        r#type: order_type_to_proto(order.order_type) as i32,
        price: crate::orderbook::string_to_uint128_option(&order.price),
        quantity: crate::orderbook::string_to_uint128_option(&order.quantity),
        remaining_quantity: crate::orderbook::string_to_uint128_option(&order.remaining_quantity),
        created_at: order.created_at,
        time_in_force: time_in_force_to_proto(order.time_in_force) as i32,
        fee_asset: "".to_string(), // Default empty string for fee_asset
    }
}

/// Converts from protocol OrderMatch to our local OrderMatch
pub fn order_match_from_proto(proto_match: &proto::OrderMatch) -> LocalOrderMatch {
    LocalOrderMatch {
        id: proto_match.id.clone(),
        market: proto_match.market.clone(),
        price: crate::orderbook::uint128_option_to_string(&proto_match.price),
        quantity: crate::orderbook::uint128_option_to_string(&proto_match.quantity),
        maker_order_id: proto_match.maker_order_id.clone(),
        taker_order_id: proto_match.taker_order_id.clone(),
        taker_side: order_side_from_proto(proto_match.taker_side()),
        timestamp: proto_match.timestamp,
    }
}

/// Converts from our local OrderMatch to protocol OrderMatch
pub fn order_match_to_proto(local_match: &LocalOrderMatch) -> proto::OrderMatch {
    proto::OrderMatch {
        id: local_match.id.clone(),
        market: local_match.market.clone(),
        price: crate::orderbook::string_to_uint128_option(&local_match.price),
        quantity: crate::orderbook::string_to_uint128_option(&local_match.quantity),
        maker_order_id: local_match.maker_order_id.clone(),
        taker_order_id: local_match.taker_order_id.clone(),
        taker_side: order_side_to_proto(local_match.taker_side) as i32,
        timestamp: local_match.timestamp,
    }
}

/// Converts from local Trade to protocol OrderMatch
pub fn trade_to_proto_match(trade: &LocalTrade) -> proto::OrderMatch {
    let local_match: LocalOrderMatch = trade.clone().into();
    order_match_to_proto(&local_match)
}

/// Converts from protocol OrderMatch to local Trade
pub fn proto_match_to_trade(proto_match: &proto::OrderMatch) -> LocalTrade {
    let local_match = order_match_from_proto(proto_match);
    local_match.into()
}

/// Converts from MarketParams to our local Market
pub fn market_from_params(market_id: &str, params: &MarketParams) -> LocalMarket {
    LocalMarket {
        id: market_id.to_string(),
        base_asset: params.base_asset.clone(),
        quote_asset: params.quote_asset.clone(),
        tick_size: params.tick_size.map_or_else(|| "0".to_string(), |v| v.to_string()),
        lot_size: params.lot_size.map_or_else(|| "0".to_string(), |v| v.to_string()),
        paused: params.paused,
    }
}

/// Converts from our local Market to MarketParams
pub fn market_to_params(market: &LocalMarket) -> MarketParams {
    MarketParams {
        base_asset: market.base_asset.clone(),
        quote_asset: market.quote_asset.clone(),
        tick_size: market.tick_size.parse::<u128>().ok(),
        lot_size: market.lot_size.parse::<u128>().ok(),
        paused: market.paused,
    }
}