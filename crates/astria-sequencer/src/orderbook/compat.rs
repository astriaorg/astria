use astria_core::protocol::orderbook::v1 as proto;

use crate::orderbook::types::{
    Order, OrderSide, OrderTimeInForce, OrderType, Market as LocalMarket, 
    OrderMatch as LocalOrderMatch, Trade as LocalTrade
};
use crate::orderbook::state_ext::MarketParams;

/// Converts from protocol OrderSide to our local OrderSide
pub fn order_side_from_proto(side: proto::OrderSide) -> OrderSide {
    match side {
        proto::OrderSide::ORDER_SIDE_BUY => OrderSide::Buy,
        proto::OrderSide::ORDER_SIDE_SELL => OrderSide::Sell,
        _ => OrderSide::Buy, // Default to Buy for unspecified
    }
}

/// Converts from our local OrderSide to protocol OrderSide
pub fn order_side_to_proto(side: OrderSide) -> proto::OrderSide {
    match side {
        OrderSide::Buy => proto::OrderSide::ORDER_SIDE_BUY,
        OrderSide::Sell => proto::OrderSide::ORDER_SIDE_SELL,
    }
}

/// Converts from protocol OrderType to our local OrderType
pub fn order_type_from_proto(order_type: proto::OrderType) -> OrderType {
    match order_type {
        proto::OrderType::ORDER_TYPE_LIMIT => OrderType::Limit,
        proto::OrderType::ORDER_TYPE_MARKET => OrderType::Market,
        _ => OrderType::Limit, // Default to Limit for unspecified
    }
}

/// Converts from our local OrderType to protocol OrderType
pub fn order_type_to_proto(order_type: OrderType) -> proto::OrderType {
    match order_type {
        OrderType::Limit => proto::OrderType::ORDER_TYPE_LIMIT,
        OrderType::Market => proto::OrderType::ORDER_TYPE_MARKET,
    }
}

/// Converts from protocol OrderTimeInForce to our local OrderTimeInForce
pub fn time_in_force_from_proto(time_in_force: proto::OrderTimeInForce) -> OrderTimeInForce {
    match time_in_force {
        proto::OrderTimeInForce::ORDER_TIME_IN_FORCE_GTC => OrderTimeInForce::GoodTillCancelled,
        proto::OrderTimeInForce::ORDER_TIME_IN_FORCE_IOC => OrderTimeInForce::ImmediateOrCancel,
        proto::OrderTimeInForce::ORDER_TIME_IN_FORCE_FOK => OrderTimeInForce::FillOrKill,
        _ => OrderTimeInForce::GoodTillCancelled, // Default to GTC for unspecified
    }
}

/// Converts from our local OrderTimeInForce to protocol OrderTimeInForce
pub fn time_in_force_to_proto(time_in_force: OrderTimeInForce) -> proto::OrderTimeInForce {
    match time_in_force {
        OrderTimeInForce::GoodTillCancelled => proto::OrderTimeInForce::ORDER_TIME_IN_FORCE_GTC,
        OrderTimeInForce::ImmediateOrCancel => proto::OrderTimeInForce::ORDER_TIME_IN_FORCE_IOC,
        OrderTimeInForce::FillOrKill => proto::OrderTimeInForce::ORDER_TIME_IN_FORCE_FOK,
    }
}

/// Converts from protocol Order to our local Order
pub fn order_from_proto(proto_order: &proto::Order) -> Order {
    Order {
        id: proto_order.id.clone(),
        owner: proto_order.owner.clone(),
        market: proto_order.market.clone(),
        side: order_side_from_proto(proto_order.side()),
        order_type: order_type_from_proto(proto_order.type_()),
        price: proto_order.price.clone(),
        quantity: proto_order.quantity.clone(),
        remaining_quantity: proto_order.remaining_quantity.clone(),
        created_at: proto_order.created_at,
        time_in_force: time_in_force_from_proto(proto_order.time_in_force()),
    }
}

/// Converts from our local Order to protocol Order
pub fn order_to_proto(order: &Order) -> proto::Order {
    proto::Order {
        id: order.id.clone(),
        owner: order.owner.clone(),
        market: order.market.clone(),
        side: order_side_to_proto(order.side) as i32,
        type_: order_type_to_proto(order.order_type) as i32,
        price: order.price.clone(),
        quantity: order.quantity.clone(),
        remaining_quantity: order.remaining_quantity.clone(),
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
        price: proto_match.price.clone(),
        quantity: proto_match.quantity.clone(),
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
        price: local_match.price.clone(),
        quantity: local_match.quantity.clone(),
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
        tick_size: params.tick_size.clone(),
        lot_size: params.lot_size.clone(),
        paused: params.paused,
    }
}

/// Converts from our local Market to MarketParams
pub fn market_to_params(market: &LocalMarket) -> MarketParams {
    MarketParams {
        base_asset: market.base_asset.clone(),
        quote_asset: market.quote_asset.clone(),
        tick_size: market.tick_size.clone(),
        lot_size: market.lot_size.clone(),
        paused: market.paused,
    }
}