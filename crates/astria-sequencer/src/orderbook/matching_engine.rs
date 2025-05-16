use std::cmp::Ordering;

use astria_core::protocol::orderbook::v1::{Order, OrderMatch, OrderSide, OrderTimeInForce, OrderType};
use cnidarium::StateDelta;
use uuid::Uuid;

use crate::orderbook::state_ext::{OrderbookError, StateReadExt, StateWriteExt};

/// A matching engine for matching orders in the order book.
#[derive(Debug, Default)]
pub struct MatchingEngine;

impl MatchingEngine {
    /// Process a new order, matching it against existing orders if possible.
    ///
    /// Returns a list of matches that occurred during processing.
    pub fn process_order<S: StateRead>(
        &self,
        state: &mut StateDelta<S>,
        order: Order,
    ) -> Result<Vec<OrderMatch>, OrderbookError> {
        match order.type_ {
            t if t == OrderType::ORDER_TYPE_LIMIT as i32 => self.process_limit_order(state, order),
            t if t == OrderType::ORDER_TYPE_MARKET as i32 => self.process_market_order(state, order),
            _ => Err(OrderbookError::InvalidOrderParameters),
        }
    }

    /// Process a limit order.
    pub(crate) fn process_limit_order<S: StateRead>(
        &self,
        state: &mut StateDelta<S>,
        order: Order,
    ) -> Result<Vec<OrderMatch>, OrderbookError> {
        let is_buy = order.side == OrderSide::ORDER_SIDE_BUY;
        let opposite_side = if is_buy {
            OrderSide::ORDER_SIDE_SELL
        } else {
            OrderSide::ORDER_SIDE_BUY
        };

        // Get matching orders on the opposite side of the book
        let mut matches = Vec::new();
        let mut remaining_quantity = order.remaining_quantity.parse::<u128>().unwrap_or(0);
        let mut matched_fully = false;

        // Get orders from the opposite side sorted by price priority
        let mut opposite_orders: Vec<Order> = state
            .get_market_orders(&order.market, Some(opposite_side))
            .collect();

        // Sort by price (best price first)
        opposite_orders.sort_by(|a, b| {
            let price_a = a.price.parse::<f64>().unwrap_or(0.0);
            let price_b = b.price.parse::<f64>().unwrap_or(0.0);
            
            match is_buy {
                // For buy orders, we want the lowest sell price first
                true => price_a.partial_cmp(&price_b).unwrap_or(Ordering::Equal),
                // For sell orders, we want the highest buy price first
                false => price_b.partial_cmp(&price_a).unwrap_or(Ordering::Equal),
            }
        });

        // Match against opposite orders
        for opposite_order in opposite_orders {
            // Check price conditions for limit orders
            let price_matches = match is_buy {
                // Buy order price >= sell order price
                true => {
                    order.price.parse::<f64>().unwrap_or(0.0) >= 
                    opposite_order.price.parse::<f64>().unwrap_or(0.0)
                },
                // Sell order price <= buy order price
                false => {
                    order.price.parse::<f64>().unwrap_or(0.0) <= 
                    opposite_order.price.parse::<f64>().unwrap_or(0.0)
                },
            };

            if !price_matches {
                continue;
            }

            let opposite_remaining = opposite_order.remaining_quantity.parse::<u128>().unwrap_or(0);
            
            if opposite_remaining == 0 {
                continue;
            }

            // Determine match quantity
            let match_qty = remaining_quantity.min(opposite_remaining);
            
            if match_qty == 0 {
                continue;
            }

            // Create the match
            let trade_id = Uuid::new_v4().to_string();
            let match_price = opposite_order.price.clone();
            
            let trade_match = OrderMatch {
                id: trade_id,
                market: order.market.clone(),
                price: match_price.clone(),
                quantity: match_qty.to_string(),
                maker_order_id: opposite_order.id.clone(),
                taker_order_id: order.id.clone(),
                taker_side: order.side,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };

            // Update the order quantities
            let new_opposite_remaining = opposite_remaining - match_qty;
            let new_order_remaining = remaining_quantity - match_qty;

            // Update the orders in the state
            state.update_order(
                &opposite_order.id,
                &new_opposite_remaining.to_string(),
            )?;
            
            // Record the trade
            state.record_trade(trade_match.clone())?;
            
            matches.push(trade_match);
            
            remaining_quantity = new_order_remaining;

            // Check if we're fully matched
            if remaining_quantity == 0 {
                matched_fully = true;
                break;
            }
        }

        // Check if we need to add the order to the book
        if !matched_fully && remaining_quantity > 0 {
            // For IOC and FOK orders, don't add to the book
            let time_in_force = order.time_in_force;
            
            if time_in_force == OrderTimeInForce::ORDER_TIME_IN_FORCE_IOC as i32 {
                // IOC - cancel any remaining quantity
                return Ok(matches);
            } else if time_in_force == OrderTimeInForce::ORDER_TIME_IN_FORCE_FOK as i32 {
                // FOK - if not fully filled, cancel the entire order
                if !matched_fully {
                    return Ok(vec![]);
                }
            }
            
            // For GTC or fully matched FOK, add/update the order
            let mut updated_order = order.clone();
            updated_order.remaining_quantity = remaining_quantity.to_string();
            state.put_order(updated_order)?;
        }

        Ok(matches)
    }

    /// Process a market order, which is executed immediately at the best available price.
    pub(crate) fn process_market_order<S: StateRead>(
        &self,
        state: &mut StateDelta<S>,
        order: Order,
    ) -> Result<Vec<OrderMatch>, OrderbookError> {
        let is_buy = order.side == OrderSide::ORDER_SIDE_BUY;
        let opposite_side = if is_buy {
            OrderSide::ORDER_SIDE_SELL
        } else {
            OrderSide::ORDER_SIDE_BUY
        };

        // Get matching orders on the opposite side of the book
        let mut matches = Vec::new();
        let mut remaining_quantity = order.remaining_quantity.parse::<u128>().unwrap_or(0);
        
        // Get orders from the opposite side sorted by price priority
        let mut opposite_orders: Vec<Order> = state
            .get_market_orders(&order.market, Some(opposite_side))
            .collect();

        // Sort by price (best price first)
        opposite_orders.sort_by(|a, b| {
            let price_a = a.price.parse::<f64>().unwrap_or(0.0);
            let price_b = b.price.parse::<f64>().unwrap_or(0.0);
            
            match is_buy {
                // For buy orders, we want the lowest sell price first
                true => price_a.partial_cmp(&price_b).unwrap_or(Ordering::Equal),
                // For sell orders, we want the highest buy price first
                false => price_b.partial_cmp(&price_a).unwrap_or(Ordering::Equal),
            }
        });

        // Match against opposite orders
        for opposite_order in opposite_orders {
            let opposite_remaining = opposite_order.remaining_quantity.parse::<u128>().unwrap_or(0);
            
            if opposite_remaining == 0 {
                continue;
            }

            // Determine match quantity
            let match_qty = remaining_quantity.min(opposite_remaining);
            
            if match_qty == 0 {
                continue;
            }

            // Create the match
            let trade_id = Uuid::new_v4().to_string();
            let match_price = opposite_order.price.clone();
            
            let trade_match = OrderMatch {
                id: trade_id,
                market: order.market.clone(),
                price: match_price.clone(),
                quantity: match_qty.to_string(),
                maker_order_id: opposite_order.id.clone(),
                taker_order_id: order.id.clone(),
                taker_side: order.side,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };

            // Update the order quantities
            let new_opposite_remaining = opposite_remaining - match_qty;
            let new_order_remaining = remaining_quantity - match_qty;

            // Update the orders in the state
            state.update_order(
                &opposite_order.id,
                &new_opposite_remaining.to_string(),
            )?;
            
            // Record the trade
            state.record_trade(trade_match.clone())?;
            
            matches.push(trade_match);
            
            remaining_quantity = new_order_remaining;

            // Check if we've fully matched
            if remaining_quantity == 0 {
                break;
            }
        }

        // Market orders are never added to the book
        Ok(matches)
    }
}