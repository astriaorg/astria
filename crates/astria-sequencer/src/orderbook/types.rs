use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use astria_core::primitive::v1::Address;

// Order side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

// Order type (limit, market, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum OrderType {
    Limit,
    Market,
}

// Order time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum OrderTimeInForce {
    GoodTillCancelled, // Order remains active until explicitly cancelled
    ImmediateOrCancel, // Execute as much as possible immediately, cancel remaining
    FillOrKill,        // Fill entire order or cancel entirely
}

// Market information
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Market {
    pub id: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub tick_size: String,
    pub lot_size: String,
    pub paused: bool,
}

// Order structure
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub owner: String,
    pub market: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: String,
    pub quantity: String,
    pub remaining_quantity: String,
    pub created_at: u64,
    pub time_in_force: OrderTimeInForce,
}

// Order book entry showing aggregate quantity at a price level
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct OrderbookEntry {
    pub price: String,
    pub quantity: String,
    pub order_count: u32,
}

// Complete order book for a market
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Orderbook {
    pub market: String,
    pub bids: Vec<OrderbookEntry>,
    pub asks: Vec<OrderbookEntry>,
}

// Record of a completed trade/match
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub market: String,
    pub price: String,
    pub quantity: String,
    pub maker_order_id: String,
    pub taker_order_id: String,
    pub maker_side: OrderSide,
    pub timestamp: u64,
}

// Compatibility layer for OrderMatch
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct OrderMatch {
    pub id: String,
    pub market: String,
    pub price: String,
    pub quantity: String,
    pub maker_order_id: String,
    pub taker_order_id: String,
    pub taker_side: OrderSide,
    pub timestamp: u64,
}

impl From<OrderMatch> for Trade {
    fn from(match_info: OrderMatch) -> Self {
        Self {
            id: match_info.id,
            market: match_info.market,
            price: match_info.price,
            quantity: match_info.quantity,
            maker_order_id: match_info.maker_order_id,
            taker_order_id: match_info.taker_order_id,
            maker_side: match opposing_side(match_info.taker_side) {
                OrderSide::Buy => OrderSide::Buy,
                OrderSide::Sell => OrderSide::Sell,
            },
            timestamp: match_info.timestamp,
        }
    }
}

impl From<Trade> for OrderMatch {
    fn from(trade: Trade) -> Self {
        Self {
            id: trade.id,
            market: trade.market,
            price: trade.price,
            quantity: trade.quantity,
            maker_order_id: trade.maker_order_id,
            taker_order_id: trade.taker_order_id,
            taker_side: match opposing_side(trade.maker_side) {
                OrderSide::Buy => OrderSide::Buy,
                OrderSide::Sell => OrderSide::Sell,
            },
            timestamp: trade.timestamp,
        }
    }
}

// Get the opposing side of an order
pub fn opposing_side(side: OrderSide) -> OrderSide {
    match side {
        OrderSide::Buy => OrderSide::Sell,
        OrderSide::Sell => OrderSide::Buy,
    }
}

// Create order action
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct CreateOrderAction {
    pub market: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: String,
    pub quantity: String,
    pub time_in_force: OrderTimeInForce,
    pub fee_asset: String,
}

// Cancel order action
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct CancelOrderAction {
    pub order_id: String,
    pub fee_asset: String,
}

// Create market action
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct CreateMarketAction {
    pub market_id: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub tick_size: String,
    pub lot_size: String,
    pub fee_asset: String,
}

// Update market action
#[derive(Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct UpdateMarketAction {
    pub market: String,
    pub tick_size: Option<String>,
    pub lot_size: Option<String>,
    pub paused: bool,
    pub fee_asset: String,
}