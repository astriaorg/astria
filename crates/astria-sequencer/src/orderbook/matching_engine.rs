use std::collections::{BTreeMap, VecDeque};

use astria_core::protocol::orderbook::v1::{Order, OrderMatch};
use cnidarium::StateRead;
use tracing::{debug, info};
use uuid::Uuid;

use crate::orderbook::state_ext::{OrderbookError, StateReadExt, StateWriteExt};
use crate::orderbook::types::{OrderSide, OrderTimeInForce, OrderType};
use crate::orderbook::utils::{parse_string_to_u128, uint128_option_to_string};

/// A structure that represents a price level in the order book
#[derive(Debug, Clone)]
struct PriceLevel {
    /// The price level
    price: u128,
    /// The orders at this price level, sorted by time priority (FIFO)
    orders: VecDeque<String>,
    /// The total quantity of all orders at this price level
    total_quantity: u128,
}

impl PriceLevel {
    fn new(price: u128) -> Self {
        Self {
            price,
            orders: VecDeque::new(),
            total_quantity: 0,
        }
    }

    fn add_order(&mut self, order_id: String, quantity: u128) {
        self.orders.push_back(order_id);
        self.total_quantity = self.total_quantity.saturating_add(quantity);
    }

    fn remove_order(&mut self, order_id: &str, quantity: u128) {
        // Remove the order ID from the queue
        let pos = self.orders.iter().position(|id| id == order_id);
        if let Some(pos) = pos {
            self.orders.remove(pos);
            self.total_quantity = self.total_quantity.saturating_sub(quantity);
        }
    }

    fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    fn first_order(&self) -> Option<&String> {
        self.orders.front()
    }
}

/// The side of the order book (bids or asks)
#[derive(Debug, Clone)]
struct OrderBookSide {
    /// The price levels in this side, ordered appropriately for the side
    /// For bids (buy orders): ordered by price descending (highest first)
    /// For asks (sell orders): ordered by price ascending (lowest first)
    price_levels: BTreeMap<u128, PriceLevel>,
    /// Whether this is the bid side (true) or ask side (false)
    is_bid_side: bool,
}

impl OrderBookSide {
    fn new(is_bid_side: bool) -> Self {
        Self {
            price_levels: BTreeMap::new(),
            is_bid_side,
        }
    }

    /// Add an order to this side of the book
    fn add_order(&mut self, order_id: String, price: u128, quantity: u128) {
        let price_level = self.price_levels
            .entry(price)
            .or_insert_with(|| PriceLevel::new(price));
        
        price_level.add_order(order_id, quantity);
    }

    /// Remove an order from this side of the book
    fn remove_order(&mut self, order_id: &str, price: u128, quantity: u128) {
        if let Some(price_level) = self.price_levels.get_mut(&price) {
            price_level.remove_order(order_id, quantity);
            
            // If the price level is now empty, remove it
            if price_level.is_empty() {
                self.price_levels.remove(&price);
            }
        }
    }

    /// Get the best price level for this side
    fn best_price(&self) -> Option<u128> {
        if self.is_bid_side {
            // For bid side, the best price is the highest (last in a BTreeMap)
            self.price_levels.keys().next_back().cloned()
        } else {
            // For ask side, the best price is the lowest (first in a BTreeMap)
            self.price_levels.keys().next().cloned()
        }
    }

    /// Check if a limit order can be matched on this side
    fn can_match_limit(&self, side: OrderSide, price: u128) -> bool {
        match side {
            OrderSide::Buy => {
                // A buy order matches if there's an ask at or below the buy price
                if !self.is_bid_side {
                    if let Some(best_ask) = self.best_price() {
                        return price >= best_ask;
                    }
                }
            }
            OrderSide::Sell => {
                // A sell order matches if there's a bid at or above the sell price
                if self.is_bid_side {
                    if let Some(best_bid) = self.best_price() {
                        return price <= best_bid;
                    }
                }
            }
        }
        false
    }

    /// Get iterator over price levels in matching order
    fn matching_prices(&self, side: OrderSide) -> Box<dyn Iterator<Item = (&u128, &PriceLevel)> + '_> {
        match (side, self.is_bid_side) {
            (OrderSide::Buy, false) => {
                // Buy order matching against ask side - iterate prices low to high
                Box::new(self.price_levels.iter())
            }
            (OrderSide::Sell, true) => {
                // Sell order matching against bid side - iterate prices high to low
                Box::new(self.price_levels.iter().rev())
            }
            _ => {
                // This shouldn't happen - we're looking at the wrong side
                Box::new(std::iter::empty())
            }
        }
    }
}

/// A matching engine for matching orders in the order book.
#[derive(Debug, Default)]
pub struct MatchingEngine;

impl MatchingEngine {
    /// Process a new order, matching it against existing orders if possible.
    ///
    /// Returns a list of matches that occurred during processing.
    pub fn process_order<S: StateRead + StateWriteExt>(
        &self,
        state: &mut S,
        order: Order,
    ) -> Result<Vec<OrderMatch>, OrderbookError> {
        info!(
            order_id = order.id,
            market = order.market,
            side = ?order.side,
            price = ?order.price,
            quantity = ?order.quantity,
            "Processing order"
        );
        
        // NOTE: In a production system, we would check the sender's balance here 
        // for SELL orders. However, this functionality is not yet implemented.
        
        // Enhanced validation and logging for SELL orders
        if order.side == astria_core::protocol::orderbook::v1::OrderSide::Sell as i32 {
            tracing::warn!(
                "💲 Processing SELL order - order_id={}, market={}, quantity={:?}",
                order.id, order.market, order.quantity
            );
            
            // Get owner information for better debugging
            let owner_str = match &order.owner {
                Some(owner) => owner.bech32m.clone(),
                None => "unknown".to_string(),
            };
            tracing::warn!("💲 Order owner: {}", owner_str);
            
            // Market validation - critical for SELL orders
            if let Some(market_params) = state.get_market_params(&order.market) {
                tracing::warn!(
                    "✅ Found market parameters: market={}, base_asset={}, quote_asset={}",
                    order.market, market_params.base_asset, market_params.quote_asset
                );
                
                // Validate that the base asset is a valid Denom
                if let Ok(denom) = market_params.base_asset.parse::<astria_core::primitive::v1::asset::Denom>() {
                    let asset_prefixed: astria_core::primitive::v1::asset::IbcPrefixed = denom.into();
                    tracing::warn!("✅ Base asset '{}' parsed as denom: {}", market_params.base_asset, asset_prefixed);
                    
                    // Verify quantity is valid
                    if let Some(quantity) = &order.quantity {
                        let qty_string = crate::orderbook::uint128_option_to_string(&Some(quantity.clone()));
                        let qty_u128 = crate::orderbook::parse_string_to_u128(&qty_string);
                        
                        if qty_u128 > 0 {
                            tracing::warn!(
                                "✅ SELL order validation passed: market={}, base_asset={}, quantity={}",
                                order.market, market_params.base_asset, qty_u128
                            );
                        } else {
                            tracing::error!("❌ SELL order has quantity of 0 or failed to parse");
                            return Err(OrderbookError::InvalidOrderParameters(
                                "SELL order must have a valid quantity greater than 0".to_string()
                            ));
                        }
                    } else {
                        tracing::error!("❌ SELL order missing quantity");
                        return Err(OrderbookError::InvalidOrderParameters(
                            "SELL order must specify a quantity".to_string()
                        ));
                    }
                } else {
                    tracing::error!(
                        "❌ Base asset '{}' in market '{}' failed to parse as a valid denom",
                        market_params.base_asset, order.market
                    );
                    
                    // Log all available markets for debugging
                    let all_markets = state.get_markets();
                    tracing::warn!("📊 Available markets: {:?}", all_markets);
                    
                    // Continue processing to allow fallback mechanisms to work
                    // We've already added fallbacks in the asset_and_amount_to_transfer method
                    tracing::warn!("⚠️ Continuing with order processing despite invalid base asset");
                }
            } else {
                tracing::error!("❌ No market parameters found for market: {}", order.market);
                
                // Log all markets for debugging
                let all_markets = state.get_markets();
                tracing::warn!("📊 Available markets: {:?}", all_markets);
                
                // Try to derive base asset from market name as fallback
                let derived_base_asset = if order.market.contains('/') {
                    let base = order.market.split('/').next().unwrap_or("ntia");
                    tracing::warn!("⚠️ Using derived base asset '{}' from market name", base);
                    base
                } else {
                    tracing::warn!("⚠️ Using fallback base asset 'ntia'");
                    "ntia"
                };
                
                // Verify derived asset is valid
                if let Ok(denom) = derived_base_asset.parse::<astria_core::primitive::v1::asset::Denom>() {
                    let asset_prefixed: astria_core::primitive::v1::asset::IbcPrefixed = denom.into();
                    tracing::warn!(
                        "✅ Derived base asset '{}' is valid: {}",
                        derived_base_asset, asset_prefixed
                    );
                } else {
                    tracing::error!("❌ Derived base asset '{}' is invalid", derived_base_asset);
                    // Continue processing to allow fallback mechanisms to work
                }
            }
            
            // Final validation step - check if quantity is valid and parse it
            let quantity = match &order.quantity {
                Some(q) => {
                    let qty_string = crate::orderbook::uint128_option_to_string(&Some(q.clone()));
                    crate::orderbook::parse_string_to_u128(&qty_string)
                },
                None => 0,
            };
            
            if quantity == 0 {
                tracing::error!("❌ SELL order has invalid or zero quantity");
                return Err(OrderbookError::InvalidOrderParameters(
                    "SELL order must have a valid quantity greater than 0".to_string()
                ));
            }
            
            tracing::warn!("💲 SELL order validation complete, continuing with processing");
        }

        // Parse order details
        let side = crate::orderbook::compat::order_side_from_proto(
            crate::orderbook::utils::order_side_from_i32(order.side)
        );
        let order_type = crate::orderbook::compat::order_type_from_proto(
            crate::orderbook::utils::order_type_from_i32(order.r#type)
        );
        let time_in_force = crate::orderbook::compat::time_in_force_from_proto(
            crate::orderbook::utils::time_in_force_from_i32(order.time_in_force)
        );
        
        // Parse numeric quantities
        let price = match &order.price {
            Some(p) => parse_string_to_u128(&uint128_option_to_string(&Some(p.clone()))),
            None => 0,
        };
        
        let quantity = match &order.quantity {
            Some(q) => parse_string_to_u128(&uint128_option_to_string(&Some(q.clone()))),
            None => return Err(OrderbookError::InvalidOrderParameters("Order quantity must be specified".to_string())),
        };
        
        let remaining_quantity = match &order.remaining_quantity {
            Some(q) => parse_string_to_u128(&uint128_option_to_string(&Some(q.clone()))),
            None => quantity, // If not specified, remaining = total
        };

        // Construct the order book from the state
        let mut order_book = self.construct_order_book(state, &order.market)?;
        
        // Process the order based on its type and time-in-force
        let matches = match order_type {
            OrderType::Limit => {
                self.process_limit_order(
                    state,
                    &order,
                    side,
                    price,
                    remaining_quantity,
                    time_in_force,
                    &mut order_book,
                )?
            }
            OrderType::Market => {
                self.process_market_order(
                    state,
                    &order,
                    side,
                    remaining_quantity,
                    time_in_force,
                    &mut order_book,
                )?
            }
        };

        // Return the matches
        Ok(matches)
    }

    /// Process a limit order
    fn process_limit_order<S: StateRead + StateWriteExt>(
        &self,
        state: &mut S,
        order: &Order,
        side: OrderSide,
        price: u128,
        quantity: u128,
        time_in_force: OrderTimeInForce,
        order_book: &mut OrderBook,
    ) -> Result<Vec<OrderMatch>, OrderbookError> {
        // Get the opposite side of the book
        let opposite_side = match side {
            OrderSide::Buy => &mut order_book.asks,
            OrderSide::Sell => &mut order_book.bids,
        };

        // Check if the order can be matched immediately
        let can_match = opposite_side.can_match_limit(side, price);
        
        // If the order can't be matched and it's FOK, reject it
        if !can_match && time_in_force == OrderTimeInForce::FillOrKill {
            debug!("FillOrKill order cannot be matched, cancelling: {}", order.id);
            return Ok(vec![]);
        }

        // Try to match the order
        let mut matches = Vec::new();
        let mut remaining_quantity = quantity;

        // Only try to match if there are possible matches
        if can_match {
            // Collect the prices and ids we need to match against first
            let mut match_queue = Vec::new();
            
            // Collect all the orders we might match against to avoid iterator borrowing issues
            for (level_price, price_level) in opposite_side.matching_prices(side) {
                // For limit orders, we only match with appropriate prices
                match side {
                    OrderSide::Buy => {
                        if *level_price > price {
                            continue;
                        }
                    }
                    OrderSide::Sell => {
                        if *level_price < price {
                            continue;
                        }
                    }
                }
                
                // Add all orders at this price level
                for order_id in &price_level.orders {
                    match_queue.push((*level_price, order_id.clone()));
                }
            }
            
            // Process the matches
            for (level_price, maker_order_id) in match_queue {
                // Stop if we're fully filled
                if remaining_quantity == 0 {
                    break;
                }
                
                // Get the maker order details
                let maker_order = match state.get_order(&maker_order_id) {
                    Some(o) => o,
                    None => {
                        // This shouldn't happen, but if it does, skip this order
                        debug!("Maker order not found: {}", maker_order_id);
                        continue;
                    }
                };

                // Get maker's remaining quantity
                let maker_remaining = parse_string_to_u128(
                    &uint128_option_to_string(&maker_order.remaining_quantity.clone())
                );
                
                // Skip orders with no remaining quantity
                if maker_remaining == 0 {
                    continue;
                }

                // Calculate the match quantity with explicit logging
                let match_quantity = std::cmp::min(remaining_quantity, maker_remaining);
                
                tracing::warn!(
                    "🔢 Order matching details - taker remaining: {}, maker remaining: {}, match quantity: {}",
                    remaining_quantity, maker_remaining, match_quantity
                );
                
                // For SELL orders, double check the quantity is correct
                if let OrderSide::Sell = side {
                    tracing::warn!("💲 SELL order matching - checking quantities carefully");
                    
                    // Verify we're not matching more than available
                    if match_quantity > maker_remaining {
                        tracing::error!(
                            "❌ Match quantity {} exceeds maker remaining {}", 
                            match_quantity, maker_remaining
                        );
                        // Correct the match quantity
                        let corrected_match = maker_remaining;
                        tracing::warn!("🔄 Correcting match quantity to: {}", corrected_match);
                    }
                    
                    if match_quantity > remaining_quantity {
                        tracing::error!(
                            "❌ Match quantity {} exceeds taker remaining {}", 
                            match_quantity, remaining_quantity
                        );
                        // Correct the match quantity in a separate variable
                        let corrected_match = remaining_quantity;
                        tracing::warn!("🔄 Correcting match quantity to: {}", corrected_match);
                    }
                }
                
                // Only create a match if the quantity is positive
                if match_quantity > 0 {
                    // Create the match
                    let order_match = OrderMatch {
                        id: Uuid::new_v4().to_string(),
                        market: order.market.clone(),
                        price: order.price.clone(), // Use the original price format
                        quantity: crate::orderbook::string_to_uint128_option(&match_quantity.to_string()),
                        maker_order_id: maker_order_id.clone(),
                        taker_order_id: order.id.clone(),
                        taker_side: order.side,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    };
                    
                    tracing::warn!(
                        "💱 Created order match: id={}, market={}, price={:?}, quantity={:?}",
                        order_match.id, order_match.market, order_match.price, order_match.quantity
                    );
                    
                    // Add the match to the list
                    matches.push(order_match);
                    
                    // Update remaining quantities
                    remaining_quantity -= match_quantity;
                    tracing::warn!("📉 Taker remaining after match: {}", remaining_quantity);
                    
                    // Update maker order
                    let new_maker_remaining = maker_remaining - match_quantity;
                    tracing::warn!("📉 Maker remaining after match: {}", new_maker_remaining);
                    
                    if new_maker_remaining == 0 {
                        // Maker order is fully filled
                        tracing::warn!("✅ Maker order fully filled, removing from book: {}", maker_order_id);
                        state.remove_order(&maker_order_id)?;
                        opposite_side.remove_order(&maker_order_id, level_price, maker_remaining);
                    } else {
                        // Maker order is partially filled
                        tracing::warn!("⏳ Maker order partially filled, updating quantity: {}", new_maker_remaining);
                        state.update_order(&maker_order_id, &new_maker_remaining.to_string())?;
                        // Note: The price level's total quantity is updated when we update the order
                    }
                } else {
                    tracing::warn!("⚠️ Skipping match with zero quantity");
                }
            }
        }

        // Handle unfilled quantity according to time in force
        if remaining_quantity > 0 {
            match time_in_force {
                OrderTimeInForce::GoodTillCancelled => {
                    // Add the remaining quantity to the book
                    tracing::warn!("📝 Adding remaining quantity {} to the orderbook for order {}", remaining_quantity, order.id);
                    
                    // For SELL orders, make sure it's added to the book correctly
                    if let OrderSide::Sell = side {
                        tracing::warn!("💲 SELL order with remaining quantity - ensuring it's stored in orderbook");
                    }
                    
                    self.add_to_book(state, order, side, price, remaining_quantity, order_book)?;
                    
                    // Double-check the order is in the book for SELL orders
                    if let OrderSide::Sell = side {
                        // Verify order is in state by reading it back
                        if let Some(stored_order) = state.get_order(&order.id) {
                            tracing::warn!("✅ Verified SELL order {} is stored in state", order.id);
                            
                            // Check remaining quantity is correct
                            let stored_remaining = crate::orderbook::uint128_option_to_string(&stored_order.remaining_quantity);
                            tracing::warn!("📊 Stored remaining quantity: {}", stored_remaining);
                        } else {
                            tracing::error!("❌ Failed to verify SELL order in state: {}", order.id);
                        }
                    }
                }
                OrderTimeInForce::ImmediateOrCancel => {
                    // Cancel the remaining quantity
                    tracing::warn!("⏱️ ImmediateOrCancel order partially filled, cancelling remainder: {}", order.id);
                }
                OrderTimeInForce::FillOrKill => {
                    // This shouldn't happen as we check at the beginning
                    tracing::warn!("⏱️ FillOrKill order cannot be fully filled, cancelling: {}", order.id);
                    return Ok(vec![]);
                }
            }
        } else {
            tracing::warn!("✅ Order fully filled: {}", order.id);
        }

        // Update the taker order if it was partially filled or remove it if fully filled
        if !matches.is_empty() {
            if remaining_quantity == 0 {
                // Order fully filled, remove it from the book
                tracing::warn!("✅ Taker order fully filled, removing from book: {}", order.id);
                state.remove_order(&order.id)?;
            } else if remaining_quantity < quantity {
                // Order partially filled, update the remaining quantity
                tracing::warn!("⏳ Taker order partially filled, updating quantity: {}", remaining_quantity);
                state.update_order(&order.id, &remaining_quantity.to_string())?;
            }
        }

        Ok(matches)
    }

    /// Process a market order
    fn process_market_order<S: StateRead + StateWriteExt>(
        &self,
        state: &mut S,
        order: &Order,
        side: OrderSide,
        quantity: u128,
        time_in_force: OrderTimeInForce,
        order_book: &mut OrderBook,
    ) -> Result<Vec<OrderMatch>, OrderbookError> {
        // Get the opposite side of the book
        let opposite_side = match side {
            OrderSide::Buy => &mut order_book.asks,
            OrderSide::Sell => &mut order_book.bids,
        };

        // Market orders always attempt to match immediately
        // For FOK orders, we need to check if we can fill the entire quantity
        if time_in_force == OrderTimeInForce::FillOrKill {
            let total_available = self.calculate_available_liquidity(opposite_side, side);
            if total_available < quantity {
                debug!("Market FillOrKill order cannot be fully filled, cancelling: {}", order.id);
                return Ok(vec![]);
            }
        }

        // Try to match the order
        let mut matches = Vec::new();
        let mut remaining_quantity = quantity;

        // Collect the prices and ids we need to match against first
        let mut match_queue = Vec::new();
        
        // Collect all the orders we might match against to avoid iterator borrowing issues
        for (level_price, price_level) in opposite_side.matching_prices(side) {
            // Add all orders at this price level
            for order_id in &price_level.orders {
                match_queue.push((*level_price, order_id.clone()));
            }
        }
        
        // Process the matches
        for (level_price, maker_order_id) in match_queue {
            // Stop if we're fully filled
            if remaining_quantity == 0 {
                break;
            }
            
            // Get the maker order details
            let maker_order = match state.get_order(&maker_order_id) {
                Some(o) => o,
                None => {
                    // This shouldn't happen, but if it does, skip this order
                    debug!("Maker order not found: {}", maker_order_id);
                    continue;
                }
            };

            // Get maker's remaining quantity
            let maker_remaining = parse_string_to_u128(
                &uint128_option_to_string(&maker_order.remaining_quantity.clone())
            );
            
            // Skip orders with no remaining quantity
            if maker_remaining == 0 {
                continue;
            }

            // Calculate the match quantity
            let match_quantity = std::cmp::min(remaining_quantity, maker_remaining);
            
            // Create the match
            let price_opt = crate::orderbook::string_to_uint128_option(&level_price.to_string());
            let order_match = OrderMatch {
                id: Uuid::new_v4().to_string(),
                market: order.market.clone(),
                price: price_opt, // Use the maker's price for market orders
                quantity: crate::orderbook::string_to_uint128_option(&match_quantity.to_string()),
                maker_order_id: maker_order_id.clone(),
                taker_order_id: order.id.clone(),
                taker_side: order.side,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };
            
            // Add the match to the list
            matches.push(order_match);
            
            // Update remaining quantities
            remaining_quantity -= match_quantity;
            
            // Update maker order
            let new_maker_remaining = maker_remaining - match_quantity;
            if new_maker_remaining == 0 {
                // Maker order is fully filled
                state.remove_order(&maker_order_id)?;
                opposite_side.remove_order(&maker_order_id, level_price, maker_remaining);
            } else {
                // Maker order is partially filled
                state.update_order(&maker_order_id, &new_maker_remaining.to_string())?;
            }
        }

        // Handle unfilled quantity according to time in force
        if remaining_quantity > 0 {
            match time_in_force {
                OrderTimeInForce::GoodTillCancelled => {
                    // Market GTC orders should fully execute or be cancelled
                    debug!("Market GTC order couldn't be fully filled, cancelling remainder: {}", order.id);
                }
                OrderTimeInForce::ImmediateOrCancel => {
                    // Cancel the remaining quantity (which is automatic for market orders)
                    debug!("Market IOC order partially filled, cancelling remainder: {}", order.id);
                }
                OrderTimeInForce::FillOrKill => {
                    // This shouldn't happen as we check at the beginning
                    debug!("Market FOK order couldn't be fully filled, should have been cancelled earlier: {}", order.id);
                    return Ok(vec![]);
                }
            }
        }

        // Update the taker order if it was partially filled or remove it if fully filled
        if !matches.is_empty() {
            if remaining_quantity == 0 {
                // Order fully filled, remove it from the book
                tracing::warn!("✅ Taker order fully filled, removing from book: {}", order.id);
                state.remove_order(&order.id)?;
            } else if remaining_quantity < quantity {
                // Order partially filled, update the remaining quantity
                tracing::warn!("⏳ Taker order partially filled, updating quantity: {}", remaining_quantity);
                state.update_order(&order.id, &remaining_quantity.to_string())?;
            }
        }

        Ok(matches)
    }

    /// Calculate the total available liquidity on a side of the book
    fn calculate_available_liquidity(&self, side: &OrderBookSide, order_side: OrderSide) -> u128 {
        let mut total = 0u128;
        for (_, price_level) in side.matching_prices(order_side) {
            total = total.saturating_add(price_level.total_quantity);
        }
        total
    }

    /// Add an order to the book
    fn add_to_book<S: StateRead + StateWriteExt>(
        &self,
        state: &mut S,
        order: &Order,
        side: OrderSide,
        price: u128,
        quantity: u128,
        order_book: &mut OrderBook,
    ) -> Result<(), OrderbookError> {
        // Enhanced logging, especially for SELL orders
        if let OrderSide::Sell = side {
            tracing::warn!("💾 Adding SELL order to orderbook: id={}, price={}, quantity={}", 
                order.id, price, quantity);
        } else {
            tracing::warn!("💾 Adding BUY order to orderbook: id={}, price={}, quantity={}", 
                order.id, price, quantity);
        }
        
        // Create a new order struct with updated remaining quantity to store
        let mut updated_order = order.clone();
        updated_order.remaining_quantity = crate::orderbook::string_to_uint128_option(&quantity.to_string());
        
        // Save the order in the state - always update or add
        match state.get_order(&order.id) {
            Some(_) => {
                tracing::warn!("🔄 Updating existing order in state with new quantity: {}", quantity);
                state.update_order(&order.id, &quantity.to_string())?;
            },
            None => {
                tracing::warn!("📝 Adding new order to state: {}", order.id);
                state.put_order(updated_order)?;
            }
        }
        
        // Get the appropriate side of the book
        let book_side = match side {
            OrderSide::Buy => {
                tracing::warn!("📊 Adding to BID side of orderbook");
                &mut order_book.bids
            },
            OrderSide::Sell => {
                tracing::warn!("📊 Adding to ASK side of orderbook");
                &mut order_book.asks
            },
        };

        // Add the order to the book
        book_side.add_order(order.id.clone(), price, quantity);
        
        // For SELL orders, verify the order was added correctly to the book
        if let OrderSide::Sell = side {
            tracing::warn!("🔍 Verifying SELL order was correctly added to book side");
            
            // Check if the order is in the correct price level
            let mut found = false;
            for (level_price, price_level) in &book_side.price_levels {
                if *level_price == price {
                    tracing::warn!("✅ Found matching price level: {}", price);
                    
                    if price_level.orders.iter().any(|id| id == &order.id) {
                        tracing::warn!("✅ Order found in correct price level");
                        found = true;
                        break;
                    }
                }
            }
            
            if !found {
                tracing::error!("❌ SELL order not found in orderbook after adding - this is a bug");
            }
        }
        
        Ok(())
    }

    /// Construct an order book from the state
    fn construct_order_book<S: StateRead>(
        &self,
        state: &S,
        market: &str,
    ) -> Result<OrderBook, OrderbookError> {
        // Get all orders for the market with enhanced logging
        tracing::warn!("📚 Constructing orderbook for market: {}", market);
        let market_orders = state.get_market_orders(market, None);
        tracing::warn!("📚 Found {} existing orders for market: {}", market_orders.len(), market);
        
        // Create the order book
        let mut order_book = OrderBook {
            bids: OrderBookSide::new(true),
            asks: OrderBookSide::new(false),
        };
        
        // Add each order to the appropriate side
        for (idx, order) in market_orders.iter().enumerate() {
            // Enhanced logging for each order
            tracing::warn!(
                "📊 Processing existing order {}/{}: id={}, market={}, side={}",
                idx + 1, market_orders.len(), order.id, order.market, order.side
            );
            
            // Parse order details
            let side = crate::orderbook::compat::order_side_from_proto(
                crate::orderbook::utils::order_side_from_i32(order.side)
            );
            
            // Parse price and remaining quantity with error handling
            let price = match &order.price {
                Some(p) => {
                    let price_str = uint128_option_to_string(&Some(p.clone()));
                    let price_u128 = parse_string_to_u128(&price_str);
                    tracing::warn!("💰 Order price: {} (from string: {})", price_u128, price_str);
                    price_u128
                },
                None => {
                    tracing::warn!("⚠️ Order has no price, using 0");
                    0
                },
            };
            
            let remaining_quantity = match &order.remaining_quantity {
                Some(q) => {
                    let qty_str = uint128_option_to_string(&Some(q.clone()));
                    let qty_u128 = parse_string_to_u128(&qty_str);
                    tracing::warn!("📏 Order remaining quantity: {} (from string: {})", qty_u128, qty_str);
                    qty_u128
                },
                None => {
                    // If remaining quantity isn't set, use the original quantity
                    match &order.quantity {
                        Some(q) => {
                            let qty_str = uint128_option_to_string(&Some(q.clone()));
                            let qty_u128 = parse_string_to_u128(&qty_str);
                            tracing::warn!("📏 Using original quantity: {} (from string: {})", qty_u128, qty_str);
                            qty_u128
                        },
                        None => {
                            tracing::warn!("⚠️ Order has no quantity, using 0");
                            0
                        },
                    }
                }
            };
            
            // Skip orders with zero remaining quantity
            if remaining_quantity == 0 {
                tracing::warn!("⚠️ Skipping order with zero remaining quantity: id={}", order.id);
                continue;
            }
            
            // Add the order to the appropriate side
            let book_side = match side {
                OrderSide::Buy => {
                    tracing::warn!("💰 Adding BUY order to orderbook: id={}, price={}, quantity={}", 
                        order.id, price, remaining_quantity);
                    &mut order_book.bids
                },
                OrderSide::Sell => {
                    tracing::warn!("💲 Adding SELL order to orderbook: id={}, price={}, quantity={}", 
                        order.id, price, remaining_quantity);
                    &mut order_book.asks
                },
            };
            
            book_side.add_order(order.id.clone(), price, remaining_quantity);
        }
        
        // Log the constructed orderbook summary
        let bid_count = order_book.bids.price_levels.len();
        let ask_count = order_book.asks.price_levels.len();
        tracing::warn!("📚 Orderbook construction complete for market {}: {} bid price levels, {} ask price levels",
            market, bid_count, ask_count);
            
        // If this is a market with no orders yet, add a special log
        if bid_count == 0 && ask_count == 0 {
            tracing::warn!("🆕 This appears to be a new market with no existing orders");
        }
        
        Ok(order_book)
    }
}

/// An order book with bid and ask sides
#[derive(Debug, Clone)]
struct OrderBook {
    /// The bid side (buy orders)
    bids: OrderBookSide,
    /// The ask side (sell orders)
    asks: OrderBookSide,
}