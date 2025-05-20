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

                // Calculate the match quantity
                let match_quantity = std::cmp::min(remaining_quantity, maker_remaining);
                
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
                    // Note: The price level's total quantity is updated when we update the order
                }
            }
        }

        // Handle unfilled quantity according to time in force
        if remaining_quantity > 0 {
            match time_in_force {
                OrderTimeInForce::GoodTillCancelled => {
                    // Add the remaining quantity to the book
                    self.add_to_book(state, order, side, price, remaining_quantity, order_book)?;
                }
                OrderTimeInForce::ImmediateOrCancel => {
                    // Cancel the remaining quantity
                    debug!("ImmediateOrCancel order partially filled, cancelling remainder: {}", order.id);
                }
                OrderTimeInForce::FillOrKill => {
                    // This shouldn't happen as we check at the beginning
                    debug!("FillOrKill order cannot be fully filled, cancelling: {}", order.id);
                    return Ok(vec![]);
                }
            }
        }

        // Update the taker order if it was partially filled
        if !matches.is_empty() && remaining_quantity < quantity {
            state.update_order(&order.id, &remaining_quantity.to_string())?;
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

        // Update the taker order if it was partially filled
        if !matches.is_empty() && remaining_quantity < quantity {
            state.update_order(&order.id, &remaining_quantity.to_string())?;
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
        // Get the appropriate side of the book
        let book_side = match side {
            OrderSide::Buy => &mut order_book.bids,
            OrderSide::Sell => &mut order_book.asks,
        };

        // Add the order to the book
        book_side.add_order(order.id.clone(), price, quantity);
        
        // Save the order in the state if it's not already there
        if state.get_order(&order.id).is_none() {
            state.put_order(order.clone())?;
        }
        
        Ok(())
    }

    /// Construct an order book from the state
    fn construct_order_book<S: StateRead>(
        &self,
        state: &S,
        market: &str,
    ) -> Result<OrderBook, OrderbookError> {
        // Get all orders for the market
        let market_orders = state.get_market_orders(market, None);
        
        // Create the order book
        let mut order_book = OrderBook {
            bids: OrderBookSide::new(true),
            asks: OrderBookSide::new(false),
        };
        
        // Add each order to the appropriate side
        for order in market_orders {
            // Parse order details
            let side = crate::orderbook::compat::order_side_from_proto(
                crate::orderbook::utils::order_side_from_i32(order.side)
            );
            
            // Parse price and remaining quantity
            let price = match &order.price {
                Some(p) => parse_string_to_u128(&uint128_option_to_string(&Some(p.clone()))),
                None => 0,
            };
            
            let remaining_quantity = match &order.remaining_quantity {
                Some(q) => parse_string_to_u128(&uint128_option_to_string(&Some(q.clone()))),
                None => {
                    // If remaining quantity isn't set, use the original quantity
                    match &order.quantity {
                        Some(q) => parse_string_to_u128(&uint128_option_to_string(&Some(q.clone()))),
                        None => 0,
                    }
                }
            };
            
            // Skip orders with zero remaining quantity
            if remaining_quantity == 0 {
                continue;
            }
            
            // Add the order to the appropriate side
            let book_side = match side {
                OrderSide::Buy => &mut order_book.bids,
                OrderSide::Sell => &mut order_book.asks,
            };
            
            book_side.add_order(order.id.clone(), price, remaining_quantity);
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