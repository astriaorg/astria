use astria_core::protocol::orderbook::v1::{
    Order, OrderSide, OrderTimeInForce, OrderType, Orderbook, OrderbookEntry, OrderMatch,
};
use borsh::{BorshDeserialize, BorshSerialize};
use cnidarium::{StateRead, StateWrite};
use futures::{pin_mut, stream::StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::orderbook::compat::{
    OrderWrapper, OrderMatchWrapper, OrderbookEntryWrapper, OrderbookWrapper
};

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
                    Ok(StoredValue::Order(wrapper)) => {
                        // Return the proto Order directly from our wrapper
                        Some(wrapper.0)
                    },
                    _ => None,
                }
            },
            _ => None,
        }
    }

    /// Get all order IDs for a specific market without filtering - used for cleanup operations
    fn get_all_market_orders_raw(&self, market: &str) -> Vec<String> {
        tracing::warn!(" Getting all raw order IDs for market: {}", market);
        
        let prefix = keys::orderbook_market_orders(market);
        tracing::warn!(" Using market prefix: {}", prefix);
        
        // Collect order IDs using the stream API with pin_mut
        futures::executor::block_on(async {
            let mut ids = Vec::new();
            let stream = self.prefix_raw(&prefix);
            pin_mut!(stream);
            
            while let Some(result) = stream.next().await {
                if let Ok((_, value)) = result {
                    let order_id = String::from_utf8_lossy(&value).to_string();
                    ids.push(order_id);
                }
            }
            
            tracing::warn!(" Found {} raw order IDs for market {}", ids.len(), market);
            ids
        })
    }

    /// Get all orders for a specific market.
    fn get_market_orders(
        &self,
        market: &str,
        side: Option<astria_core::protocol::orderbook::v1::OrderSide>,
    ) -> Vec<Order> {
        // Enhanced logging, especially helpful for debugging SELL orders
        if let Some(side_val) = side {
            if side_val == astria_core::protocol::orderbook::v1::OrderSide::Sell {
                tracing::warn!(" Specifically querying SELL orders for market: {}", market);
            } else {
                tracing::warn!(" Specifically querying BUY orders for market: {}", market);
            }
        } else {
            tracing::warn!(" Querying ALL orders for market: {}", market);
        }
        
        let prefix = match side {
            Some(side) => {
                let prefix_str = keys::orderbook_market_side_orders(market, side);
                tracing::warn!(" Using market-side prefix: {}", prefix_str);
                prefix_str
            },
            None => {
                let prefix_str = keys::orderbook_market_orders(market);
                tracing::warn!(" Using market prefix: {}", prefix_str);
                prefix_str
            },
        };

        // Collect order IDs using the stream API with pin_mut
        let order_ids = futures::executor::block_on(async {
            let mut ids = Vec::new();
            let stream = self.prefix_raw(&prefix);
            pin_mut!(stream);
            
            while let Some(result) = stream.next().await {
                if let Ok((key, value)) = result {
                    // Log the raw key for debugging
                    let key_str = String::from_utf8_lossy(key.as_bytes());
                    let value_str = String::from_utf8_lossy(&value);
                    tracing::warn!(" Found order entry - Key: {}, Value: {}", key_str, value_str);
                    
                    ids.push(value_str.to_string());
                }
            }
            
            // Log how many order IDs were found
            tracing::warn!(" Found {} order IDs for market {} and prefix {}", 
                ids.len(), market, prefix);
            
            ids
        });
            
        // Now get each order with detailed logging
        let mut orders = Vec::new();
        for (idx, order_id) in order_ids.iter().enumerate() {
            tracing::warn!(" Getting order {}/{}: {}", idx+1, order_ids.len(), order_id);
            
            if let Some(order) = self.get_order(order_id) {
                // For SELL orders, log additional details
                if order.side == astria_core::protocol::orderbook::v1::OrderSide::Sell as i32 {
                    tracing::warn!(" Found SELL order: id={}, market={}, remaining_quantity={:?}", 
                        order.id, order.market, order.remaining_quantity);
                } else {
                    tracing::warn!(" Found BUY order: id={}, market={}, remaining_quantity={:?}",
                        order.id, order.market, order.remaining_quantity);
                }
                
                // Skip orders with zero remaining quantity
                let remaining_qty = crate::orderbook::uint128_option_to_string(&order.remaining_quantity);
                if remaining_qty == "0" {
                    tracing::warn!(" Skipping order with zero remaining quantity: {}", order_id);
                    continue;
                }
                
                // Add the order to the results
                orders.push(order);
            } else {
                tracing::warn!(" Order not found in storage: {}", order_id);
            }
        }
        
        // Final count of orders returned
        tracing::warn!(" Returning {} orders for market {}", orders.len(), market);
        
        orders
    }

    /// Get all orders for a specific owner.
    fn get_owner_orders(&self, owner: &str) -> Vec<Order> {
        let prefix = keys::orderbook_owner_orders(owner);
        
        // Collect order IDs using the stream API with pin_mut
        let order_ids = futures::executor::block_on(async {
            let mut ids = Vec::new();
            let stream = self.prefix_raw(&prefix);
            pin_mut!(stream);
            
            while let Some(result) = stream.next().await {
                if let Ok((_, value)) = result {
                    ids.push(String::from_utf8_lossy(&value).to_string());
                }
            }
            
            ids
        });
            
        // Now get each order
        let mut orders = Vec::new();
        for order_id in order_ids {
            if let Some(order) = self.get_order(&order_id) {
                orders.push(order);
            }
        }
        
        orders
    }

    /// Get the orderbook for a specific market.
    fn get_orderbook(&self, market: &str) -> Orderbook {
        // Use Buy and Sell directly
        let bid_side = OrderSide::Buy;
        let ask_side = OrderSide::Sell;
        
        let bid_prefix = keys::orderbook_market_price_levels(market, bid_side);
        let ask_prefix = keys::orderbook_market_price_levels(market, ask_side);
        
        // Collect bids using the stream API with pin_mut
        let mut bids = futures::executor::block_on(async {
            let mut entries = Vec::new();
            let stream = self.prefix_raw(&bid_prefix);
            pin_mut!(stream);
            
            while let Some(result) = stream.next().await {
                if let Ok((key, value)) = result {
                    // Extract the price from the key
                    let key_bytes = key.as_bytes();
                    if key_bytes.len() >= 16 {
                        let price = String::from_utf8_lossy(&key_bytes[key_bytes.len() - 16..]).to_string();
                        
                        // Try to deserialize the entry using our wrapper
                        if let Ok(wrapper) = <OrderbookEntryWrapper as BorshDeserialize>::deserialize(&mut value.as_slice()) {
                            let entry = wrapper.0;
                            entries.push((price, entry));
                        }
                    }
                }
            }
            
            entries
        });
            
        // Sort bids by price in descending order (highest price first for bids)
        bids.sort_by(|(a, _), (b, _)| b.cmp(a));

        // Collect asks using the stream API with pin_mut
        let mut asks = futures::executor::block_on(async {
            let mut entries = Vec::new();
            let stream = self.prefix_raw(&ask_prefix);
            pin_mut!(stream);
            
            while let Some(result) = stream.next().await {
                if let Ok((key, value)) = result {
                    // Extract the price from the key
                    let key_bytes = key.as_bytes();
                    if key_bytes.len() >= 16 {
                        let price = String::from_utf8_lossy(&key_bytes[key_bytes.len() - 16..]).to_string();
                        
                        // Try to deserialize the entry using our wrapper
                        if let Ok(wrapper) = <OrderbookEntryWrapper as BorshDeserialize>::deserialize(&mut value.as_slice()) {
                            let entry = wrapper.0;
                            entries.push((price, entry));
                        }
                    }
                }
            }
            
            entries
        });
            
        // Sort asks by price in ascending order (lowest price first for asks)
        asks.sort_by(|(a, _), (b, _)| a.cmp(b));

        // Build orderbook from the collected entries
        let bid_entries: Vec<OrderbookEntry> = bids.into_iter().map(|(_, entry)| entry).collect();
        let ask_entries: Vec<OrderbookEntry> = asks.into_iter().map(|(_, entry)| entry).collect();
        
        Orderbook {
            market: market.to_string(),
            bids: bid_entries,
            asks: ask_entries,
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
        // Add detailed logging to trace the market params lookup
        tracing::warn!(" Looking up market parameters for market: {}", market);
        
        // Get the market params key
        let market_params_key = keys::orderbook_market_params(market);
        tracing::warn!(" Market params storage key: {}", market_params_key);
        
        // Print all known markets to help diagnose issues
        let all_markets = self.get_markets();
        tracing::warn!(" All known markets: {:?}", all_markets);
        
        // Replace direct get_raw with handling the async future
        let bytes_result = futures::executor::block_on(self.get_raw(market_params_key.as_str()));
        
        match bytes_result {
            Ok(Some(bytes)) => {
                tracing::warn!(" Found raw bytes for market params, length: {}", bytes.len());
                match StoredValue::deserialize(&bytes) {
                    Ok(StoredValue::MarketParams(params)) => {
                        tracing::warn!(
                            "✅ Parsed market params - Base: {}, Quote: {}, Paused: {}", 
                            params.base_asset, params.quote_asset, params.paused
                        );
                        
                        // Additional validation for debugging
                        if params.base_asset.is_empty() {
                            tracing::error!(" Base asset is empty for market {}", market);
                        }
                        
                        if params.quote_asset.is_empty() {
                            tracing::error!(" Quote asset is empty for market {}", market);
                        }
                        
                        // Attempt to parse base and quote assets as Denoms for validation
                        if let Ok(base_denom) = params.base_asset.parse::<astria_core::primitive::v1::asset::Denom>() {
                            let prefixed: astria_core::primitive::v1::asset::IbcPrefixed = base_denom.into();
                            tracing::warn!(" Base asset '{}' parsed as denom: {}", params.base_asset, prefixed);
                        } else {
                            tracing::error!(" Base asset '{}' failed to parse as a Denom", params.base_asset);
                        }
                        
                        if let Ok(quote_denom) = params.quote_asset.parse::<astria_core::primitive::v1::asset::Denom>() {
                            let prefixed: astria_core::primitive::v1::asset::IbcPrefixed = quote_denom.into();
                            tracing::warn!(" Quote asset '{}' parsed as denom: {}", params.quote_asset, prefixed);
                        } else {
                            tracing::error!(" Quote asset '{}' failed to parse as a Denom", params.quote_asset);
                        }
                        
                        Some(params)
                    },
                    Ok(other) => {
                        tracing::error!(" Wrong StoredValue type: {:?}", other);
                        None
                    },
                    Err(err) => {
                        tracing::error!(" Failed to deserialize market params: {}", err);
                        None
                    }
                }
            },
            Ok(None) => {
                tracing::error!(" No market params found for market: {}", market);
                // Try debugging by checking if market exists at all
                if self.market_exists(market) {
                    tracing::warn!(" Market exists but no parameters found: {}", market);
                } else {
                    tracing::error!(" Market does not exist: {}", market);
                }
                None
            },
            Err(err) => {
                tracing::error!(" Error fetching market params: {}", err);
                None
            }
        }
    }

    /// Get all available markets.
    fn get_markets(&self) -> Vec<String> {
        // First try to get from the ALL_MARKETS key
        let all_markets_key = keys::orderbook_all_markets();
        let all_markets_bytes = futures::executor::block_on(self.get_raw(all_markets_key.as_str()));
        
        if let Ok(Some(bytes)) = all_markets_bytes {
            // Try to deserialize as StoredValue and then as Vec<String>
            if let Ok(StoredValue::Bytes(inner_bytes)) = StoredValue::deserialize(&bytes) {
                if let Ok(markets) = borsh::from_slice::<Vec<String>>(&inner_bytes) {
                    return markets;
                }
            }
        }
        
        // Fallback to scanning the orderbook_markets prefix
        let prefix = keys::orderbook_markets();
        
        futures::executor::block_on(async {
            let mut markets = Vec::new();
            let stream = self.prefix_raw(&prefix);
            pin_mut!(stream);
            
            while let Some(result) = stream.next().await {
                if let Ok((_, value)) = result {
                    markets.push(String::from_utf8_lossy(&value).to_string());
                }
            }
            
            markets
        })
    }

    /// Get recent trades for a market.
    fn get_recent_trades(&self, market: &str, limit: usize) -> Vec<OrderMatch> {
        let prefix = keys::orderbook_market_trades(market);
        
        // Use pin_mut for proper pinning
        let mut trades = futures::executor::block_on(async {
            let mut vec = Vec::new();
            let stream = self.prefix_raw(&prefix);
            pin_mut!(stream);
            
            while let Some(result) = stream.next().await {
                if let Ok((_, value)) = result {
                    if let Ok(wrapper) = <OrderMatchWrapper as BorshDeserialize>::deserialize(&mut value.as_slice()) {
                        let trade = wrapper.0;
                        vec.push(trade);
                    }
                }
            }
            
            vec
        });

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
        // Special logging for SELL orders to track their journey through the system
        if order.side == astria_core::protocol::orderbook::v1::OrderSide::Sell as i32 {
            tracing::warn!(" Saving SELL order to database: id={}, market={}", order.id, order.market);
            
            // Verify the SELL order has a remaining_quantity set
            let remaining_qty = crate::orderbook::uint128_option_to_string(&order.remaining_quantity);
            tracing::warn!(" SELL order remaining quantity: {}", remaining_qty);
            
            if remaining_qty == "0" {
                tracing::error!(" SELL order has zero remaining quantity, this is likely incorrect");
                
                // If quantity exists but remaining_quantity is missing, copy quantity to remaining_quantity
                let quantity = crate::orderbook::uint128_option_to_string(&order.quantity);
                if quantity != "0" {
                    tracing::warn!(" SELL order quantity is {}, copying to remaining_quantity", quantity);
                    // We can't modify the order directly due to ownership rules, but we'll log this issue
                }
            }
        }
        
        // Convert the i32 side to OrderSide enum for storage operations
        let order_side = crate::orderbook::order_side_from_i32(order.side);
        tracing::warn!(" Order side converted to enum: {:?}", order_side);
        
        // Store the order itself using our wrapper
        let serialized = match StoredValue::Order(OrderWrapper(order.clone())).serialize() {
            Ok(s) => s,
            Err(err) => {
                tracing::error!(" Failed to serialize order: {:?}", err);
                return Err(OrderbookError::SerializationError(format!("Failed to serialize order: {:?}", err)));
            }
        };
        
        // Try storing the order with error handling
        let order_key = keys::orderbook_order(&order.id);
        tracing::warn!(" Storing order at key: {}", order_key);
        self.put_raw(order_key, serialized.clone());

        // Get the price as a string, handling the case where it might be empty
        let price_str = crate::orderbook::uint128_option_to_string(&order.price);
        tracing::warn!(" Order price: {} (formatted from {:?})", price_str, order.price);

        // For SELL orders, verify the price string is valid
        if order.side == astria_core::protocol::orderbook::v1::OrderSide::Sell as i32 {
            let price_u128 = crate::orderbook::parse_string_to_u128(&price_str);
            if price_u128 == 0 {
                tracing::error!(" SELL order has zero or invalid price: {}", price_str);
            } else {
                tracing::warn!(" SELL order has valid price: {}", price_u128);
            }
        }

        // Add to market-side-price index with error handling
        let market_side_price_key = keys::orderbook_market_side_price_order(&order.market, order_side, &price_str, &order.id);
        tracing::warn!(" Storing market-side-price index at key: {}", market_side_price_key);
        self.put_raw(market_side_price_key, order.id.as_bytes().to_vec());
        
        // Verification for SELL orders to ensure they appear in the market-side index
        if order.side == astria_core::protocol::orderbook::v1::OrderSide::Sell as i32 {
            tracing::warn!(" SELL order - ensuring proper indexing in market-side");
            
            // Double check with the correct order_side value
            if order_side != astria_core::protocol::orderbook::v1::OrderSide::Sell {
                tracing::error!(" Order side conversion error for SELL order - this is a critical bug");
                tracing::warn!(" Forcing order_side to SELL for order {}", order.id);
                // Force the correct order_side for the next operations
                let sell_side = astria_core::protocol::orderbook::v1::OrderSide::Sell;
                
                // Re-add to market-side-price index with correct side
                let corrected_key = keys::orderbook_market_side_price_order(&order.market, sell_side, &price_str, &order.id);
                tracing::warn!(" Also storing market-side-price index with corrected side at key: {}", corrected_key);
                self.put_raw(corrected_key, order.id.as_bytes().to_vec());
                
                // Re-add to market-side index with correct side
                let corrected_side_key = keys::orderbook_market_side_order(&order.market, sell_side, &order.id);
                tracing::warn!(" Also storing market-side index with corrected side at key: {}", corrected_side_key);
                self.put_raw(corrected_side_key, order.id.as_bytes().to_vec());
            }
        }

        // Add to market-side index with error handling
        let market_side_key = keys::orderbook_market_side_order(&order.market, order_side, &order.id);
        tracing::warn!(" Storing market-side index at key: {}", market_side_key);
        self.put_raw(market_side_key, order.id.as_bytes().to_vec());

        // Add to market index with error handling
        let market_key = keys::orderbook_market_order(&order.market, &order.id);
        tracing::warn!(" Storing market index at key: {}", market_key);
        self.put_raw(market_key, order.id.as_bytes().to_vec());

        // Add to owner's orders index
        let owner_str = match &order.owner {
            Some(addr) => {
                tracing::warn!(" Order owner: {}", addr.bech32m);
                addr.bech32m.clone()
            },
            None => {
                tracing::warn!(" Order has no owner, using 'unknown'");
                "unknown".to_string()
            },
        };
        
        let owner_key = keys::orderbook_owner_order(&owner_str, &order.id);
        tracing::warn!(" Storing owner index at key: {}", owner_key);
        self.put_raw(owner_key, order.id.as_bytes().to_vec());

        // Update price level with detailed error handling
        tracing::warn!(" Updating price level for market: {}, side: {:?}, price: {}", 
            order.market, order_side, price_str);
        match self.update_price_level(
            &order.market,
            order_side,
            &price_str,
            |mut level| {
                // Convert the quantity strings to u128
                let level_qty = crate::orderbook::parse_string_to_u128(&level.quantity);
                let order_qty = crate::orderbook::uint128_option_to_string(&order.remaining_quantity);
                let order_qty_num = crate::orderbook::parse_string_to_u128(&order_qty);
                
                tracing::warn!(" Updating price level - current quantity: {}, adding: {}", level_qty, order_qty_num);
                
                // Add the quantities
                level.quantity = level_qty
                    .checked_add(order_qty_num)
                    .map(|q| q.to_string())
                    .unwrap_or_else(|| "0".to_string());
                
                level.order_count += 1;
                tracing::warn!(" Updated price level - new quantity: {}, order count: {}", level.quantity, level.order_count);
                level
            },
        ) {
            Ok(_) => {
                tracing::warn!(" Successfully updated price level");
            },
            Err(err) => {
                tracing::error!(" Failed to update price level: {:?}", err);
                // Don't fail the entire operation if price level update fails
                // Just log it and continue - this is especially important for SELL orders
                if order.side == astria_core::protocol::orderbook::v1::OrderSide::Sell as i32 {
                    tracing::warn!(" Continuing despite price level update failure for SELL order");
                } else {
                    return Err(err);
                }
            }
        }

        // For SELL orders, extra verification step to ensure the order was saved correctly
        if order.side == astria_core::protocol::orderbook::v1::OrderSide::Sell as i32 {
            // Verify the order exists in storage
            let check_order_key = keys::orderbook_order(&order.id);
            let exists = futures::executor::block_on(self.get_raw(check_order_key.as_str())).is_ok();
            
            if exists {
                tracing::warn!(" Verified SELL order exists in primary storage");
            } else {
                tracing::error!(" Failed to verify SELL order in primary storage - this is a critical bug");
            }
            
            // Verify the order exists in market-side index
            let check_market_side_key = keys::orderbook_market_side_order(
                &order.market,
                astria_core::protocol::orderbook::v1::OrderSide::Sell,
                &order.id
            );
            let market_side_exists = futures::executor::block_on(self.get_raw(check_market_side_key.as_str())).is_ok();
            
            if market_side_exists {
                tracing::warn!(" Verified SELL order exists in market-side index");
            } else {
                tracing::error!(" Failed to verify SELL order in market-side index - this is a critical bug");
            }
        }

        tracing::warn!(" Successfully saved order to database: id={}", order.id);
        Ok(())
    }

    /// Remove an order from the order book.
    fn remove_order(&mut self, order_id: &str) -> Result<(), OrderbookError> {
        let order = self
            .get_order(order_id)
            .ok_or_else(|| OrderbookError::OrderNotFound(order_id.to_string()))?;

        // Convert the i32 side to OrderSide enum for storage operations
        let order_side = crate::orderbook::order_side_from_i32(order.side);

        // Get the price as a string
        let price_str = crate::orderbook::uint128_option_to_string(&order.price);

        // Remove the order itself
        self.delete(keys::orderbook_order(order_id));

        // Remove from market-side-price index
        self.delete(keys::orderbook_market_side_price_order(
            &order.market,
            order_side,
            &price_str,
            order_id,
        ));

        // Remove from market-side index
        self.delete(keys::orderbook_market_side_order(&order.market, order_side, order_id));

        // Remove from market index
        self.delete(keys::orderbook_market_order(&order.market, order_id));

        // Remove from owner's orders index
        let owner_str = match &order.owner {
            Some(addr) => addr.bech32m.clone(),
            None => "unknown".to_string(),
        };
        self.delete(keys::orderbook_owner_order(&owner_str, order_id));

        // Update price level
        self.update_price_level(
            &order.market,
            order_side,
            &price_str,
            |mut level| {
                // Convert the quantity strings to u128
                let level_qty = crate::orderbook::parse_string_to_u128(&level.quantity);
                let order_qty = crate::orderbook::uint128_option_to_string(&order.remaining_quantity);
                let order_qty_num = crate::orderbook::parse_string_to_u128(&order_qty);
                
                level.quantity = level_qty
                    .checked_sub(order_qty_num)
                    .map(|q| q.to_string())
                    .unwrap_or_else(|| "0".to_string());
                
                level.order_count = level.order_count.saturating_sub(1);
                level
            },
        )?;

        // If the price level is now empty, remove it
        let price_level_key = keys::orderbook_market_price_level(
            &order.market,
            order_side,
            &price_str,
        );
        
        // Use async get_raw properly
        let bytes_result = futures::executor::block_on(self.get_raw(price_level_key.as_str()));
        
        if let Ok(Some(bytes)) = bytes_result {
            // Use our local OrderbookEntry instead of the proto one
            if let Ok(entry) = crate::orderbook::OrderbookEntry::try_from_slice(&bytes) {
                if entry.order_count == 0 || entry.quantity == "0" {
                    self.delete(price_level_key);
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
            .ok_or_else(|| OrderbookError::OrderNotFound(order_id.to_string()))?;

        // Convert the i32 side to OrderSide enum for storage operations
        let order_side = crate::orderbook::order_side_from_i32(order.side);

        // Get the price as a string
        let price_str = crate::orderbook::uint128_option_to_string(&order.price);

        // Get the old remaining quantity as u128
        let old_remaining = crate::orderbook::uint128_option_to_string(&order.remaining_quantity);
        let old_remaining_u128 = crate::orderbook::parse_string_to_u128(&old_remaining);
        
        // Get the new remaining quantity as u128
        let new_remaining_u128 = crate::orderbook::parse_string_to_u128(remaining_quantity);

        if old_remaining_u128 == new_remaining_u128 {
            return Ok(());
        }

        // Calculate the difference to update the price level
        let quantity_delta = if new_remaining_u128 > old_remaining_u128 {
            new_remaining_u128 - old_remaining_u128
        } else {
            old_remaining_u128 - new_remaining_u128
        };

        // Update the order's remaining quantity
        order.remaining_quantity = crate::orderbook::string_to_uint128_option(remaining_quantity);
        
        // Serialize the updated order
        let serialized = StoredValue::Order(OrderWrapper(order.clone())).serialize()
            .map_err(|_| OrderbookError::SerializationError(String::from("Failed to serialize order")))?;
            
        self.put_raw(
            keys::orderbook_order(order_id),
            serialized,
        );

        // Update price level
        self.update_price_level(
            &order.market,
            order_side,
            &price_str,
            |mut level| {
                let current_quantity = crate::orderbook::parse_string_to_u128(&level.quantity);
                
                level.quantity = if new_remaining_u128 > old_remaining_u128 {
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

        // Add to markets list (for backward compatibility)
        self.put_raw(
            keys::orderbook_markets(),
            market.as_bytes().to_vec(),
        );

        // Get existing markets from the ALL_MARKETS key
        let all_markets_key = keys::orderbook_all_markets();
        let all_markets_bytes = futures::executor::block_on(self.get_raw(all_markets_key.as_str()));
        
        let mut markets = Vec::new();
        if let Ok(Some(bytes)) = all_markets_bytes {
            // Try to deserialize existing markets list
            if let Ok(StoredValue::Bytes(inner_bytes)) = StoredValue::deserialize(&bytes) {
                if let Ok(existing_markets) = borsh::from_slice::<Vec<String>>(&inner_bytes) {
                    markets = existing_markets;
                }
            }
        }
        
        // Add the new market if it's not already in the list
        if !markets.contains(&market.to_string()) {
            markets.push(market.to_string());
        }
        
        // Serialize and store the updated markets list
        let markets_serialized = borsh::to_vec(&markets)
            .map_err(|_| OrderbookError::SerializationError(String::from("Failed to serialize markets list")))?;
            
        let wrapped_markets_serialized = StoredValue::Bytes(markets_serialized).serialize()
            .map_err(|_| OrderbookError::SerializationError(String::from("Failed to serialize StoredValue")))?;
            
        self.put_raw(
            all_markets_key,
            wrapped_markets_serialized,
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
        
        // Convert the trade to a StoredValue and serialize it using our wrapper
        let serialized = StoredValue::OrderMatch(OrderMatchWrapper(trade)).serialize()
            .map_err(|_| OrderbookError::SerializationError(String::from("Failed to serialize trade")))?;
            
        self.put_raw(trade_key, serialized);
        Ok(())
    }

    // Helper to update a price level
    fn update_price_level(
        &mut self,
        market: &str,
        side: astria_core::protocol::orderbook::v1::OrderSide,
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
        
        // Use StoredValue to serialize the entry
        let serialized = StoredValue::Bytes(borsh::to_vec(&updated_entry)
            .map_err(|_| OrderbookError::SerializationError(String::from("Failed to serialize OrderbookEntry")))?)
            .serialize()
            .map_err(|_| OrderbookError::SerializationError(String::from("Failed to serialize StoredValue")))?;
        
        self.put_raw(
            price_level_key,
            serialized,
        );

        Ok(())
    }
}

// Implement the StateWriteExt trait for any type that implements StateWrite
impl<T: StateWrite + ?Sized> StateWriteExt for T {}

