use astria_core::protocol::orderbook::v1::{Order, OrderMatch as Trade};
use prost::Message;
use serde::{Deserialize, Serialize};

/// A list of markets
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarketList {
    pub markets: Vec<String>,
}

impl MarketList {
    pub fn decode(bytes: &[u8]) -> Result<Self, prost::DecodeError> {
        // Try to decode directly as a JSON array of strings
        match serde_json::from_slice::<Vec<String>>(bytes) {
            Ok(markets) => {
                return Ok(Self { markets });
            }
            Err(e) => {
                // Fall back to trying other formats
                tracing::warn!("Failed to decode markets as JSON array: {}", e);
            }
        }
        
        // Try to decode as JSON object with a markets field
        match serde_json::from_slice::<Self>(bytes) {
            Ok(market_list) => {
                return Ok(market_list);
            }
            Err(e) => {
                // Last fallback
                tracing::warn!("Failed to decode markets as JSON object: {}", e);
                Err(prost::DecodeError::new(format!("Failed to decode MarketList: {}", e)))
            }
        }
    }
}

/// A list of orders
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrderList {
    pub orders: Vec<Order>,
}

impl OrderList {
    pub fn decode(bytes: &[u8]) -> Result<Self, prost::DecodeError> {
        // Try to decode as protobuf message for a single Order
        if let Ok(order) = Order::decode(&mut bytes.as_ref()) {
            return Ok(Self { orders: vec![order] });
        }
        
        // Try to decode as JSON
        match serde_json::from_slice::<Vec<Order>>(bytes) {
            Ok(orders) => {
                return Ok(Self { orders });
            }
            Err(e) => {
                tracing::warn!("Failed to decode orders as JSON: {}", e);
                Err(prost::DecodeError::new(format!("Failed to decode OrderList: {}", e)))
            }
        }
    }
}

/// A list of trades
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TradeList {
    pub trades: Vec<Trade>,
}

impl TradeList {
    pub fn decode(bytes: &[u8]) -> Result<Self, prost::DecodeError> {
        // Try to decode as protobuf message for a single Trade
        if let Ok(trade) = Trade::decode(&mut bytes.as_ref()) {
            return Ok(Self { trades: vec![trade] });
        }
        
        // Try to decode as JSON
        match serde_json::from_slice::<Vec<Trade>>(bytes) {
            Ok(trades) => {
                return Ok(Self { trades });
            }
            Err(e) => {
                tracing::warn!("Failed to decode trades as JSON: {}", e);
                Err(prost::DecodeError::new(format!("Failed to decode TradeList: {}", e)))
            }
        }
    }
}