use std::collections::{BTreeMap, BTreeSet};

use astria_core::protocol::orderbook::v1::{
    Order, OrderSide, OrderTimeInForce, OrderType, Orderbook, OrderbookEntry, OrderMatch,
};
use borsh::{BorshDeserialize, BorshSerialize};
use cnidarium::{StateRead, StateWrite};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::storage::{keys, StoredValue};

/// A trait providing methods for reading order book data from storage.
pub trait StateReadExt: StateRead {
    /// Get an order by its ID.
    fn get_order(&self, order_id: &str) -> Option<Order> {
        self.get_raw(&keys::orderbook_order(order_id))
            .and_then(|bytes| match StoredValue::deserialize(&bytes) {
                Ok(StoredValue::Order(order)) => Some(order),
                _ => None,
            })
    }

    /// Get all orders for a specific market.
    fn get_market_orders(
        &self,
        market: &str,
        side: Option<OrderSide>,
    ) -> impl Iterator<Item = Order> + '_ {
        let prefix = match side {
            Some(side) => keys::orderbook_market_side_orders(market, side),
            None => keys::orderbook_market_orders(market),
        };

        self.prefix_raw(prefix.as_bytes())
            .filter_map(move |(_, value)| {
                let order_id = String::from_utf8_lossy(&value).to_string();
                self.get_order(&order_id)
            })
    }

    /// Get all orders for a specific owner.
    fn get_owner_orders(&self, owner: &str) -> impl Iterator<Item = Order> + '_ {
        self.prefix_raw(keys::orderbook_owner_orders(owner).as_bytes())
            .filter_map(move |(_, value)| {
                let order_id = String::from_utf8_lossy(&value).to_string();
                self.get_order(&order_id)
            })
    }

    /// Get the orderbook for a specific market.
    fn get_orderbook(&self, market: &str) -> Orderbook {
        let mut bids = self
            .prefix_raw(keys::orderbook_market_price_levels(market, OrderSide::ORDER_SIDE_BUY).as_bytes())
            .filter_map(|(key, value)| {
                let price = String::from_utf8_lossy(&key[key.len() - 16..]).to_string();
                let entry = OrderbookEntry::try_from_slice(&value).ok()?;
                Some((price, entry))
            })
            .collect::<Vec<_>>();

        // Sort bids by price in descending order
        bids.sort_by(|(a, _), (b, _)| b.cmp(a));

        let mut asks = self
            .prefix_raw(
                keys::orderbook_market_price_levels(market, OrderSide::ORDER_SIDE_SELL).as_bytes(),
            )
            .filter_map(|(key, value)| {
                let price = String::from_utf8_lossy(&key[key.len() - 16..]).to_string();
                let entry = OrderbookEntry::try_from_slice(&value).ok()?;
                Some((price, entry))
            })
            .collect::<Vec<_>>();

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
        self.get_raw(&keys::orderbook_market(market)).is_some()
    }

    /// Get market parameters.
    fn get_market_params(&self, market: &str) -> Option<MarketParams> {
        self.get_raw(&keys::orderbook_market_params(market))
            .and_then(|bytes| match StoredValue::deserialize(&bytes) {
                Ok(StoredValue::MarketParams(params)) => Some(params),
                _ => None,
            })
    }

    /// Get all available markets.
    fn get_markets(&self) -> impl Iterator<Item = String> + '_ {
        self.prefix_raw(keys::orderbook_markets().as_bytes())
            .map(|(_, value)| String::from_utf8_lossy(&value).to_string())
    }

    /// Get recent trades for a market.
    fn get_recent_trades(&self, market: &str, limit: usize) -> Vec<OrderMatch> {
        let mut trades = self
            .prefix_raw(keys::orderbook_market_trades(market).as_bytes())
            .filter_map(|(_, value)| OrderMatch::try_from_slice(&value).ok())
            .collect::<Vec<_>>();

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
        self.put_raw(
            &keys::orderbook_order(&order.id),
            &StoredValue::Order(order.clone()).serialize(),
        );

        // Add to market-side-price index
        self.put_raw(
            &keys::orderbook_market_side_price_order(&order.market, order.side, &order.price.to_string(), &order.id),
            &order.id.as_bytes(),
        );

        // Add to market-side index
        self.put_raw(
            &keys::orderbook_market_side_order(&order.market, order.side, &order.id),
            &order.id.as_bytes(),
        );

        // Add to market index
        self.put_raw(
            &keys::orderbook_market_order(&order.market, &order.id),
            &order.id.as_bytes(),
        );

        // Add to owner's orders index
        self.put_raw(
            &keys::orderbook_owner_order(&order.owner.to_string(), &order.id),
            &order.id.as_bytes(),
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
        self.delete_raw(&keys::orderbook_order(order_id));

        // Remove from market-side-price index
        self.delete_raw(&keys::orderbook_market_side_price_order(
            &order.market,
            order.side,
            &order.price.to_string(),
            order_id,
        ));

        // Remove from market-side index
        self.delete_raw(&keys::orderbook_market_side_order(&order.market, order.side, order_id));

        // Remove from market index
        self.delete_raw(&keys::orderbook_market_order(&order.market, order_id));

        // Remove from owner's orders index
        self.delete_raw(&keys::orderbook_owner_order(&order.owner.to_string(), order_id));

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
            return Err(OrderbookError::MarketAlreadyExists);
        }

        // Store the market
        self.put_raw(
            &keys::orderbook_market(market),
            &market.as_bytes(),
        );

        // Add to markets list
        self.put_raw(
            &keys::orderbook_markets(),
            &market.as_bytes(),
        );

        // Store market parameters
        self.put_raw(
            &keys::orderbook_market_params(market),
            &StoredValue::MarketParams(params).serialize(),
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
            return Err(OrderbookError::MarketNotFound);
        }

        self.put_raw(
            &keys::orderbook_market_params(market),
            &StoredValue::MarketParams(params).serialize(),
        );

        Ok(())
    }

    /// Record a trade in the order book.
    fn record_trade(&mut self, trade: OrderMatch) -> Result<(), OrderbookError> {
        let trade_key = keys::orderbook_market_trade(&trade.market, &trade.id);
        self.put_raw(&trade_key, &trade.try_to_vec().map_err(|_| OrderbookError::SerializationError)?);
        Ok(())
    }

    // Helper to update a price level
    fn update_price_level(
        &mut self,
        market: &str,
        side: OrderSide,
        price: &str,
        update_fn: impl FnOnce(OrderbookEntry) -> OrderbookEntry,
    ) -> Result<(), OrderbookError> {
        let price_level_key = keys::orderbook_market_price_level(market, side, price);
        
        let entry = if let Some(bytes) = self.get_raw(&price_level_key) {
            match OrderbookEntry::try_from_slice(&bytes) {
                Ok(entry) => entry,
                Err(_) => OrderbookEntry {
                    price: price.to_string(),
                    quantity: "0".to_string(),
                    order_count: 0,
                },
            }
        } else {
            OrderbookEntry {
                price: price.to_string(),
                quantity: "0".to_string(),
                order_count: 0,
            }
        };

        let updated_entry = update_fn(entry);
        self.put_raw(
            &price_level_key,
            &updated_entry.try_to_vec().map_err(|_| OrderbookError::SerializationError)?,
        );

        Ok(())
    }
}

// Implement the StateWriteExt trait for any type that implements StateWrite
impl<T: StateWrite + ?Sized> StateWriteExt for T {}

/// Parameters for a market in the order book.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct MarketParams {
    pub base_asset: String,
    pub quote_asset: String,
    pub tick_size: String,
    pub lot_size: String,
    pub paused: bool,
}

/// Errors that can occur when interacting with the order book.
#[derive(Debug, Error)]
pub enum OrderbookError {
    #[error("Order not found")]
    OrderNotFound,
    #[error("Market not found")]
    MarketNotFound,
    #[error("Market already exists")]
    MarketAlreadyExists,
    #[error("Invalid order parameters")]
    InvalidOrderParameters,
    #[error("Serialization error")]
    SerializationError,
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Order book operation failed: {0}")]
    OperationFailed(String),
}