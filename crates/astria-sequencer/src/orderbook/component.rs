use std::{collections::HashMap, sync::Arc};

use astria_core::{
    primitive::v1::Address,
    protocol::orderbook::v1::{
        CreateMarket, CreateOrder, CancelOrder, UpdateMarket,
        Order, OrderSide, OrderTimeInForce, OrderType, OrderMatch,
    },
};
use cnidarium::{StateRead, StateWrite};
use tendermint::abci::{self, request, Event, EventAttribute};
use thiserror::Error;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::orderbook::MatchingEngine;

use crate::{
    checked_actions::{
        self, ActionRef, CheckedAction, CheckedActionError,
    },
    component::Component,
    orderbook::state_ext::{MarketParams, OrderbookError, StateReadExt, StateWriteExt},
};

/// The order book component for the Astria sequencer.
#[derive(Debug, Default)]
pub struct OrderbookComponent;

#[async_trait::async_trait]
impl Component for OrderbookComponent {
    type AppState = ();

    /// Initialize the order book component at genesis.
    async fn init_chain<S: StateWrite>(state: S, _app_state: &Self::AppState) -> astria_eyre::eyre::Result<()> {
        info!("Initializing OrderbookComponent");
        Ok(())
    }

    /// Process begin_block events for the order book.
    async fn begin_block<S: StateWrite + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &abci::request::BeginBlock,
    ) -> astria_eyre::eyre::Result<()> {
        Ok(())
    }

    /// Process end_block events for the order book.
    async fn end_block<S: StateWrite + 'static>(
        _state: &mut Arc<S>,
        _end_block: &abci::request::EndBlock,
    ) -> astria_eyre::eyre::Result<()> {
        Ok(())
    }
}

/// Error type for the order book component.
#[derive(Debug, Error)]
pub enum OrderbookComponentError {
    #[error("Orderbook error: {0}")]
    OrderbookError(#[from] OrderbookError),
    #[error("Failed to check action: {0}")]
    CheckedActionError(#[from] CheckedActionError),
    #[error(transparent)]
    Other(#[from] astria_eyre::Report),
}

/// A checked create order action.
#[derive(Debug)]
pub struct CheckedCreateOrder {
    /// The sender of the transaction
    pub sender: Address,
    /// The market to create the order in
    pub market: String,
    /// The side of the order
    pub side: OrderSide,
    /// The type of the order
    pub order_type: OrderType,
    /// The price of the order
    pub price: String,
    /// The quantity of the order
    pub quantity: String,
    /// The time in force of the order
    pub time_in_force: OrderTimeInForce,
    /// The asset used to pay fees
    pub fee_asset: String,
}

// Create types that match the protobuf definitions
pub type CreateOrder = astria_core::protocol::orderbook::v1::CreateOrder;
pub type CancelOrder = astria_core::protocol::orderbook::v1::CancelOrder;
pub type CreateMarket = astria_core::protocol::orderbook::v1::CreateMarket;
pub type UpdateMarket = astria_core::protocol::orderbook::v1::UpdateMarket;

impl CheckedAction for CheckedCreateOrder {
    fn execute<S: StateRead>(self, state: &mut StateDelta<S>) -> Result<Vec<Event>, CheckedActionError> {
        debug!(?self, "Executing CheckedCreateOrder");

        // Check if market exists
        if !state.market_exists(&self.market) {
            return Err(CheckedActionError::Custom(format!(
                "Market '{}' does not exist",
                self.market
            )));
        }

        // Get market parameters
        let market_params = state
            .get_market_params(&self.market)
            .ok_or_else(|| {
                CheckedActionError::Custom(format!("Market parameters not found for '{}'", self.market))
            })?;

        // Check if market is paused
        if market_params.paused {
            return Err(CheckedActionError::Custom(format!(
                "Market '{}' is paused",
                self.market
            )));
        }

        // Validate price and quantity against market parameters
        // TODO: Add proper validation logic for tick size and lot size

        // Create new order ID
        let order_id = Uuid::new_v4().to_string();

        // Create the order
        let order = Order {
            id: order_id.clone(),
            owner: self.sender.to_string(),
            market: self.market.clone(),
            side: self.side,
            type_: self.order_type as i32,
            price: self.price.clone(),
            quantity: self.quantity.clone(),
            remaining_quantity: self.quantity.clone(),
            created_at: chrono::Utc::now().timestamp() as u64,
            time_in_force: self.time_in_force as i32,
            fee_asset: self.fee_asset,
        };

        // Add order to the order book and match it
        match state.put_order(order.clone()) {
            Ok(_) => {
                // Create the matching engine
                let matching_engine = MatchingEngine::default();
                
                // Process the order with the matching engine
                let matches = matching_engine.process_order(state, order.clone())
                    .map_err(|err| {
                        CheckedActionError::Custom(format!(
                            "Failed to process order: {}",
                            err
                        ))
                    })?;
                
                // Create events for the order creation and matches
                let mut events = vec![
                    Event::new(
                        "order_created",
                        vec![
                            EventAttribute::new("order_id", &order_id),
                            EventAttribute::new("market", &self.market),
                            EventAttribute::new("side", format!("{:?}", self.side)),
                            EventAttribute::new("price", &self.price),
                            EventAttribute::new("quantity", &self.quantity),
                        ],
                    )
                ];
                
                // Add events for each match
                for trade_match in matches {
                    events.push(Event::new(
                        "order_matched",
                        vec![
                            EventAttribute::new("match_id", &trade_match.id),
                            EventAttribute::new("market", &trade_match.market),
                            EventAttribute::new("price", &trade_match.price),
                            EventAttribute::new("quantity", &trade_match.quantity),
                            EventAttribute::new("maker_order_id", &trade_match.maker_order_id),
                            EventAttribute::new("taker_order_id", &trade_match.taker_order_id),
                        ],
                    ));
                }

                Ok(events)
            }
            Err(err) => Err(CheckedActionError::Custom(format!(
                "Failed to add order to order book: {}",
                err
            ))),
        }
    }
}

/// A checked cancel order action.
#[derive(Debug)]
pub struct CheckedCancelOrder {
    /// The sender of the transaction
    pub sender: Address,
    /// The ID of the order to cancel
    pub order_id: String,
    /// The asset used to pay fees
    pub fee_asset: String,
}

impl CheckedAction for CheckedCancelOrder {
    fn execute<S: StateRead>(self, state: &mut StateDelta<S>) -> Result<Vec<Event>, CheckedActionError> {
        debug!(?self, "Executing CheckedCancelOrder");

        // Get the order
        let order = state
            .get_order(&self.order_id)
            .ok_or_else(|| {
                CheckedActionError::Custom(format!("Order '{}' not found", self.order_id))
            })?;

        // Check if the sender is the owner of the order
        if order.owner != self.sender.to_string() {
            return Err(CheckedActionError::Custom(
                "Only the owner can cancel their order".to_string(),
            ));
        }

        // Remove the order from the order book
        match state.remove_order(&self.order_id) {
            Ok(_) => {
                // Create event for the order cancellation
                let event = Event::new(
                    "order_cancelled",
                    vec![
                        EventAttribute::new("order_id", &self.order_id),
                        EventAttribute::new("market", &order.market),
                    ],
                );

                Ok(vec![event])
            }
            Err(err) => Err(CheckedActionError::Custom(format!(
                "Failed to remove order from order book: {}",
                err
            ))),
        }
    }
}

/// A checked create market action.
#[derive(Debug)]
pub struct CheckedCreateMarket {
    /// The sender of the transaction
    pub sender: Address,
    /// The market identifier
    pub market: String,
    /// The base asset of the market
    pub base_asset: String,
    /// The quote asset of the market
    pub quote_asset: String,
    /// The minimum price increment
    pub tick_size: String,
    /// The minimum quantity increment
    pub lot_size: String,
    /// The asset used to pay fees
    pub fee_asset: String,
}

impl CheckedAction for CheckedCreateMarket {
    fn execute<S: StateRead>(self, state: &mut StateDelta<S>) -> Result<Vec<Event>, CheckedActionError> {
        debug!(?self, "Executing CheckedCreateMarket");

        // Check if market already exists
        if state.market_exists(&self.market) {
            return Err(CheckedActionError::Custom(format!(
                "Market '{}' already exists",
                self.market
            )));
        }

        // Create market parameters
        let market_params = MarketParams {
            base_asset: self.base_asset.clone(),
            quote_asset: self.quote_asset.clone(),
            tick_size: self.tick_size.clone(),
            lot_size: self.lot_size.clone(),
            paused: false,
        };

        // Add market to the order book
        match state.add_market(&self.market, market_params) {
            Ok(_) => {
                // Create event for the market creation
                let event = Event::new(
                    "market_created",
                    vec![
                        EventAttribute::new("market", &self.market),
                        EventAttribute::new("base_asset", &self.base_asset),
                        EventAttribute::new("quote_asset", &self.quote_asset),
                    ],
                );

                Ok(vec![event])
            }
            Err(err) => Err(CheckedActionError::Custom(format!(
                "Failed to add market to order book: {}",
                err
            ))),
        }
    }
}

/// A checked update market action.
#[derive(Debug)]
pub struct CheckedUpdateMarket {
    /// The sender of the transaction
    pub sender: Address,
    /// The market identifier
    pub market: String,
    /// The new minimum price increment (if provided)
    pub tick_size: Option<String>,
    /// The new minimum quantity increment (if provided)
    pub lot_size: Option<String>,
    /// Whether the market is paused
    pub paused: bool,
    /// The asset used to pay fees
    pub fee_asset: String,
}

impl CheckedAction for CheckedUpdateMarket {
    fn execute<S: StateRead>(self, state: &mut StateDelta<S>) -> Result<Vec<Event>, CheckedActionError> {
        debug!(?self, "Executing CheckedUpdateMarket");

        // Check if market exists
        if !state.market_exists(&self.market) {
            return Err(CheckedActionError::Custom(format!(
                "Market '{}' does not exist",
                self.market
            )));
        }

        // Get current market parameters
        let mut market_params = state
            .get_market_params(&self.market)
            .ok_or_else(|| {
                CheckedActionError::Custom(format!("Market parameters not found for '{}'", self.market))
            })?;

        // Update market parameters
        if let Some(tick_size) = self.tick_size {
            market_params.tick_size = tick_size;
        }

        if let Some(lot_size) = self.lot_size {
            market_params.lot_size = lot_size;
        }

        market_params.paused = self.paused;

        // Update market parameters in state
        match state.update_market_params(&self.market, market_params) {
            Ok(_) => {
                // Create event for the market update
                let event = Event::new(
                    "market_updated",
                    vec![
                        EventAttribute::new("market", &self.market),
                        EventAttribute::new("paused", self.paused.to_string()),
                    ],
                );

                Ok(vec![event])
            }
            Err(err) => Err(CheckedActionError::Custom(format!(
                "Failed to update market parameters: {}",
                err
            ))),
        }
    }
}

