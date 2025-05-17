use std::collections::{BTreeMap, BTreeSet};

use astria_core::protocol::orderbook::v1::{
    Order, OrderSide, OrderTimeInForce, OrderType, Orderbook, OrderbookEntry, OrderMatch,
};
use borsh::{BorshDeserialize, BorshSerialize};
use cnidarium::{StateRead, StateWrite};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::storage::{keys, StoredValue};

/// Parameters for a market in the orderbook
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct MarketParams {
    /// Base asset of the market (e.g., "BTC")
    pub base_asset: String,
    /// Quote asset of the market (e.g., "USD")
    pub quote_asset: String,
    /// Minimum price increment (tick size)
    pub tick_size: Option<u128>,
    /// Minimum quantity increment (lot size)
    pub lot_size: Option<u128>,
    /// Whether the market is paused
    pub paused: bool,
}

/// Errors that can occur in orderbook operations
#[derive(Debug, Error)]
pub enum OrderbookError {
    #[error("Invalid order parameters: {0}")]
    InvalidOrderParameters(String),
    
    #[error("Market not found: {0}")]
    MarketNotFound(String),
    
    #[error("Market already exists: {0}")]
    MarketAlreadyExists(String),
    
    #[error("Order not found: {0}")]
    OrderNotFound(String),
    
    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),
    
    #[error("Invalid price level: {0}")]
    InvalidPriceLevel(String),
    
    #[error("Market is paused: {0}")]
    MarketPaused(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// A trait providing methods for reading order book data from storage.
pub trait StateReadExt: StateRead {
    /// Get an order by its ID.
    fn get_order(&self, order_id: &str) -> Option<Order> {
        // Replace direct get_raw with handling the async future
        let bytes_result = futures::executor::block_on(self.get_raw(keys::orderbook_order(order_id).as_str()));
        
        match bytes_result {
            Ok(Some(bytes)) => {
                match StoredValue::deserialize(&bytes) {
                    Ok(StoredValue::Order(order)) => Some(order),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    /// Get all orders for a specific market.
    fn get_market_orders(
        &self,
        market: &str,
        side: Option<OrderSide>,
    ) -> Vec<Order> {
        let prefix = match side {
            Some(side) => keys::orderbook_market_side_orders(market, side),
            None => keys::orderbook_market_orders(market),
        };

        // Use collect to gather all values, then process them
        let order_ids: Vec<String> = self.prefix_raw(&prefix)
            .filter_map(|result| {
                result.ok().map(|(_, value)| {
                    String::from_utf8_lossy(&value).to_string()
                })
            })
            .collect();
            
        // Now get each order
        order_ids.into_iter()
            .filter_map(|order_id| self.get_order(&order_id))
            .collect()
    }

    /// Get all orders for a specific owner.
    fn get_owner_orders(&self, owner: &str) -> Vec<Order> {
        let prefix = keys::orderbook_owner_orders(owner);
        
        // Use collect to gather all values, then process them
        let order_ids: Vec<String> = self.prefix_raw(&prefix)
            .filter_map(|result| {
                result.ok().map(|(_, value)| {
                    String::from_utf8_lossy(&value).to_string()
                })
            })
            .collect();
            
        // Now get each order
        order_ids.into_iter()
            .filter_map(|order_id| self.get_order(&order_id))
            .collect()
    }

    /// Get the orderbook for a specific market.
    fn get_orderbook(&self, market: &str) -> Orderbook {
        let bid_prefix = keys::orderbook_market_price_levels(market, OrderSide::ORDER_SIDE_BUY);
        let ask_prefix = keys::orderbook_market_price_levels(market, OrderSide::ORDER_SIDE_SELL);
        
        // Process bids
        let mut bids: Vec<(String, OrderbookEntry)> = self.prefix_raw(&bid_prefix)
            .filter_map(|result| {
                result.ok().and_then(|(key, value)| {
                    // Extract the price from the key, assuming it's in the last 16 bytes
                    let key_bytes = key.as_bytes();
                    if key_bytes.len() >= 16 {
                        let price = String::from_utf8_lossy(&key_bytes[key_bytes.len() - 16..]).to_string();
                        // Use BorshDeserialize directly
                        <OrderbookEntry as BorshDeserialize>::deserialize(&mut value.as_slice()).ok().map(|entry| (price, entry))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Sort bids by price in descending order
        bids.sort_by(|(a, _), (b, _)| b.cmp(a));

        // Process asks
        let mut asks: Vec<(String, OrderbookEntry)> = self.prefix_raw(&ask_prefix)
            .filter_map(|result| {
                result.ok().and_then(|(key, value)| {
                    // Extract the price from the key, assuming it's in the last 16 bytes
                    let key_bytes = key.as_bytes();
                    if key_bytes.len() >= 16 {
                        let price = String::from_utf8_lossy(&key_bytes[key_bytes.len() - 16..]).to_string();
                        // Use BorshDeserialize directly
                        <OrderbookEntry as BorshDeserialize>::deserialize(&mut value.as_slice()).ok().map(|entry| (price, entry))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Sort asks by price in ascending order
        asks.sort_by(|(a, _), (b, _)| a.cmp(b));

        Orderbook {
            market: market.to_string(),
            bids: bids.into_iter().map(|(_, entry)| entry).collect(),
            asks: asks.into_iter().map(|(_, entry)| entry).collect(),
        }
    }

    /// Check if a market exists.
    fn market_exists(&self, market: &str) -> bool {
        match futures::executor::block_on(self.get_raw(keys::orderbook_market(market).as_str())) {
            Ok(Some(_)) => true,
            _ => false,
        }
    }

    /// Get market parameters.
    fn get_market_params(&self, market: &str) -> Option<MarketParams> {
        // Replace direct get_raw with handling the async future
        let bytes_result = futures::executor::block_on(self.get_raw(keys::orderbook_market_params(market).as_str()));
        
        match bytes_result {
            Ok(Some(bytes)) => {
                match StoredValue::deserialize(&bytes) {
                    Ok(StoredValue::MarketParams(params)) => Some(params),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    /// Get all available markets.
    fn get_markets(&self) -> Vec<String> {
        let prefix = keys::orderbook_markets();
        
        self.prefix_raw(&prefix)
            .filter_map(|result| {
                result.ok().map(|(_, value)| String::from_utf8_lossy(&value).to_string())
            })
            .collect()
    }

    /// Get recent trades for a market.
    fn get_recent_trades(&self, market: &str, limit: usize) -> Vec<OrderMatch> {
        let prefix = keys::orderbook_market_trades(market);
        
        let mut trades: Vec<OrderMatch> = self.prefix_raw(&prefix)
            .filter_map(|result| {
                result.ok().and_then(|(_, value)| {
                    <OrderMatch as BorshDeserialize>::deserialize(&mut value.as_slice()).ok()
                })
            })
            .collect();

        // Sort by timestamp descending (most recent first)
        trades.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Limit the number of trades returned
        trades.truncate(limit);
        trades
    }
}

// Implement the StateReadExt trait for any type that implements StateRead
impl<T: StateRead + ?Sized> StateReadExt for T {}

/// A trait providing methods for writing order book data to storage.
pub trait StateWriteExt: StateWrite {
    /// Add a new order to the order book.
    fn put_order(&mut self, order: Order) -> Result<(), OrderbookError> {
        // Store the order itself
        let serialized = StoredValue::Order(order.clone()).serialize()
            .map_err(|_| OrderbookError::SerializationError(String::from("Failed to serialize order")))?;
            
        self.put_raw(
            keys::orderbook_order(&order.id),
            serialized,
        );

        // Add to market-side-price index
        self.put_raw(
            keys::orderbook_market_side_price_order(&order.market, order.side, &order.price.to_string(), &order.id),
            order.id.as_bytes().to_vec(),
        );

        // Add to market-side index
        self.put_raw(
            keys::orderbook_market_side_order(&order.market, order.side, &order.id),
            order.id.as_bytes().to_vec(),
        );

        // Add to market index
        self.put_raw(
            keys::orderbook_market_order(&order.market, &order.id),
            order.id.as_bytes().to_vec(),
        );

        // Add to owner's orders index
        self.put_raw(
            keys::orderbook_owner_order(&order.owner.to_string(), &order.id),
            order.id.as_bytes().to_vec(),
        );

        // Update price level
        self.update_price_level(
            &order.market,
            order.side,
            &order.price.to_string(),
            |level| {
                level.quantity = level
                    .quantity
                    .to_string()
                    .parse::<u128>()
                    .unwrap_or(0)
                    .checked_add(order.quantity.to_string().parse::<u128>().unwrap_or(0))
                    .map(|q| q.to_string())
                    .unwrap_or_else(|| "0".to_string());
                level.order_count += 1;
                level
            },
        )?;

        Ok(())
    }

    /// Remove an order from the order book.
    fn remove_order(&mut self, order_id: &str) -> Result<(), OrderbookError> {
        let order = self
            .get_order(order_id)
            .ok_or(OrderbookError::OrderNotFound)?;

        // Remove the order itself
        self.delete_raw(keys::orderbook_order(order_id));

        // Remove from market-side-price index
        self.delete_raw(keys::orderbook_market_side_price_order(
            &order.market,
            order.side,
            &order.price.to_string(),
            order_id,
        ));

        // Remove from market-side index
        self.delete_raw(keys::orderbook_market_side_order(&order.market, order.side, order_id));

        // Remove from market index
        self.delete_raw(keys::orderbook_market_order(&order.market, order_id));

        // Remove from owner's orders index
        self.delete_raw(keys::orderbook_owner_order(&order.owner.to_string(), order_id));

        // Update price level
        self.update_price_level(
            &order.market,
            order.side,
            &order.price.to_string(),
            |level| {
                let current_quantity = level
                    .quantity
                    .to_string()
                    .parse::<u128>()
                    .unwrap_or(0);
                let order_quantity = order
                    .remaining_quantity
                    .to_string()
                    .parse::<u128>()
                    .unwrap_or(0);
                
                level.quantity = current_quantity
                    .checked_sub(order_quantity)
                    .map(|q| q.to_string())
                    .unwrap_or_else(|| "0".to_string());
                
                level.order_count = level.order_count.saturating_sub(1);
                level
            },
        )?;

        // If the price level is now empty, remove it
        let price_level_key = keys::orderbook_market_price_level(
            &order.market,
            order.side,
            &order.price.to_string(),
        );
        if let Some(bytes) = self.get_raw(&price_level_key) {
            if let Ok(entry) = OrderbookEntry::try_from_slice(&bytes) {
                if entry.order_count == 0 || entry.quantity == "0".to_string() {
                    self.delete_raw(&price_level_key);
                }
            }
        }

        Ok(())
    }

    /// Update an order in the order book.
    fn update_order(
        &mut self,
        order_id: &str,
        remaining_quantity: &str,
    ) -> Result<(), OrderbookError> {
        let mut order = self
            .get_order(order_id)
            .ok_or(OrderbookError::OrderNotFound)?;

        let old_remaining = order
            .remaining_quantity
            .to_string()
            .parse::<u128>()
            .unwrap_or(0);
        let new_remaining = remaining_quantity.parse::<u128>().unwrap_or(0);

        if old_remaining == new_remaining {
            return Ok(());
        }

        // Calculate the difference to update the price level
        let quantity_delta = if new_remaining > old_remaining {
            new_remaining - old_remaining
        } else {
            old_remaining - new_remaining
        };

        // Update the order
        order.remaining_quantity = remaining_quantity.to_string();
        self.put_raw(
            &keys::orderbook_order(order_id),
            &StoredValue::Order(order.clone()).serialize(),
        );

        // Update price level
        self.update_price_level(
            &order.market,
            order.side,
            &order.price.to_string(),
            |level| {
                let current_quantity = level
                    .quantity
                    .to_string()
                    .parse::<u128>()
                    .unwrap_or(0);
                
                level.quantity = if new_remaining > old_remaining {
                    current_quantity
                        .checked_add(quantity_delta)
                        .map(|q| q.to_string())
                        .unwrap_or_else(|| current_quantity.to_string())
                } else {
                    current_quantity
                        .checked_sub(quantity_delta)
                        .map(|q| q.to_string())
                        .unwrap_or_else(|| "0".to_string())
                };
                
                level
            },
        )?;

        Ok(())
    }

    /// Add a market to the order book.
    fn add_market(&mut self, market: &str, params: MarketParams) -> Result<(), OrderbookError> {
        if self.market_exists(market) {
            return Err(OrderbookError::MarketAlreadyExists(format!("Market {} already exists", market)));
        }

        // Store the market
        self.put_raw(
            keys::orderbook_market(market),
            market.as_bytes().to_vec(),
        );

        // Add to markets list
        self.put_raw(
            keys::orderbook_markets(),
            market.as_bytes().to_vec(),
        );

        // Store market parameters
        let serialized = StoredValue::MarketParams(params).serialize()
            .map_err(|_| OrderbookError::SerializationError(String::from("Failed to serialize market parameters")))?;
            
        self.put_raw(
            keys::orderbook_market_params(market),
            serialized,
        );

        Ok(())
    }

    /// Update market parameters.
    fn update_market_params(
        &mut self,
        market: &str,
        params: MarketParams,
    ) -> Result<(), OrderbookError> {
        if !self.market_exists(market) {
            return Err(OrderbookError::MarketNotFound(format!("Market {} not found", market)));
        }

        let serialized = StoredValue::MarketParams(params).serialize()
            .map_err(|_| OrderbookError::SerializationError(String::from("Failed to serialize market parameters")))?;
            
        self.put_raw(
            keys::orderbook_market_params(market),
            serialized,
        );

        Ok(())
    }

    /// Record a trade in the order book.
    fn record_trade(&mut self, trade: OrderMatch) -> Result<(), OrderbookError> {
        let trade_key = keys::orderbook_market_trade(&trade.market, &trade.id);
        
        // Use borsh serialization instead of serde
        let buf = borsh::to_vec(&trade)
            .map_err(|_| OrderbookError::SerializationError(String::from("Failed to serialize trade")))?;
            
        self.put_raw(trade_key, buf);
        Ok(())
    }

    // Helper to update a price level
    fn update_price_level(
        &mut self,
        market: &str,
        side: OrderSide,
        price: &str,
        update_fn: impl FnOnce(crate::orderbook::OrderbookEntry) -> crate::orderbook::OrderbookEntry,
    ) -> Result<(), OrderbookError> {
        let price_level_key = keys::orderbook_market_price_level(market, side, price);
        
        // Use our local OrderbookEntry instead of the proto one
        let entry = if let Ok(Some(bytes)) = futures::executor::block_on(self.get_raw(price_level_key.as_str())) {
            match crate::orderbook::OrderbookEntry::try_from_slice(&bytes) {
                Ok(entry) => entry,
                Err(_) => crate::orderbook::OrderbookEntry {
                    price: price.to_string(),
                    quantity: "0".to_string(),
                    order_count: 0,
                },
            }
        } else {
            crate::orderbook::OrderbookEntry {
                price: price.to_string(),
                quantity: "0".to_string(),
                order_count: 0,
            }
        };

        let updated_entry = update_fn(entry);
        // Use borsh serialization instead of serde
        let buf = borsh::to_vec(&updated_entry)
            .map_err(|_| OrderbookError::SerializationError(String::from("Failed to serialize OrderbookEntry")))?;
        
        self.put_raw(
            price_level_key,
            buf,
        );

        Ok(())
    }
}

// Implement the StateWriteExt trait for any type that implements StateWrite
impl<T: StateWrite + ?Sized> StateWriteExt for T {}

