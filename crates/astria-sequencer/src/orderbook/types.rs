use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
// Fix import paths
use astria_core::generated::astria::primitive::v1::Address;
use astria_core::generated::astria::primitive::v1::Uint128;

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

// Type alias for component module
pub type TimeInForce = OrderTimeInForce;

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

// Orderbook depth level for aggregated view
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderbookDepthLevel {
    pub price: Option<Uint128>,
    pub quantity: Option<Uint128>,
    pub order_count: u32,
}

// Manual implementation of BorshSerialize for OrderbookDepthLevel
impl BorshSerialize for OrderbookDepthLevel {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Convert Uint128 to string for serialization
        let price_str = self.price.as_ref().map(|p| format!("{}{}", p.hi, p.lo)).unwrap_or_default();
        let quantity_str = self.quantity.as_ref().map(|q| format!("{}{}", q.hi, q.lo)).unwrap_or_default();
        
        // Serialize the string representations and order count
        BorshSerialize::serialize(&price_str, writer)?;
        BorshSerialize::serialize(&quantity_str, writer)?;
        BorshSerialize::serialize(&self.order_count, writer)?;
        
        Ok(())
    }
}

// Manual implementation of BorshDeserialize for OrderbookDepthLevel
impl BorshDeserialize for OrderbookDepthLevel {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        // Deserialize the string representations and order count
        let price_str: String = BorshDeserialize::deserialize_reader(reader)?;
        let quantity_str: String = BorshDeserialize::deserialize_reader(reader)?;
        let order_count: u32 = BorshDeserialize::deserialize_reader(reader)?;
        
        // Convert strings to Uint128 if not empty
        let price = if !price_str.is_empty() {
            // Parse the string as a u128 and convert to hi/lo
            let price_val = price_str.parse::<u128>().unwrap_or_default();
            Some(Uint128 { 
                lo: price_val as u64,
                hi: (price_val >> 64) as u64,
            })
        } else {
            None
        };
        
        let quantity = if !quantity_str.is_empty() {
            // Parse the string as a u128 and convert to hi/lo
            let qty_val = quantity_str.parse::<u128>().unwrap_or_default();
            Some(Uint128 { 
                lo: qty_val as u64,
                hi: (qty_val >> 64) as u64,
            })
        } else {
            None
        };
        
        Ok(Self {
            price,
            quantity,
            order_count,
        })
    }
}

// Aggregated orderbook depth
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderbookDepth {
    pub market: String,
    pub bids: Vec<OrderbookDepthLevel>,
    pub asks: Vec<OrderbookDepthLevel>,
}

// Manual implementation of BorshSerialize for OrderbookDepth
impl BorshSerialize for OrderbookDepth {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Serialize the market string
        BorshSerialize::serialize(&self.market, writer)?;
        
        // Serialize the bids vector length
        BorshSerialize::serialize(&(self.bids.len() as u32), writer)?;
        // Serialize each bid
        for bid in &self.bids {
            BorshSerialize::serialize(bid, writer)?;
        }
        
        // Serialize the asks vector length
        BorshSerialize::serialize(&(self.asks.len() as u32), writer)?;
        // Serialize each ask
        for ask in &self.asks {
            BorshSerialize::serialize(ask, writer)?;
        }
        
        Ok(())
    }
}

// Manual implementation of BorshDeserialize for OrderbookDepth
impl BorshDeserialize for OrderbookDepth {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        // Deserialize the market string
        let market: String = BorshDeserialize::deserialize_reader(reader)?;
        
        // Deserialize the bids vector
        let bids_len: u32 = BorshDeserialize::deserialize_reader(reader)?;
        let mut bids = Vec::with_capacity(bids_len as usize);
        for _ in 0..bids_len {
            bids.push(OrderbookDepthLevel::deserialize_reader(reader)?);
        }
        
        // Deserialize the asks vector
        let asks_len: u32 = BorshDeserialize::deserialize_reader(reader)?;
        let mut asks = Vec::with_capacity(asks_len as usize);
        for _ in 0..asks_len {
            asks.push(OrderbookDepthLevel::deserialize_reader(reader)?);
        }
        
        Ok(Self {
            market,
            bids,
            asks,
        })
    }
}