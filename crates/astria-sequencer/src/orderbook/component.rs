use std::{collections::HashMap, sync::Arc};

use astria_core::{
    primitive::v1::Address,
    protocol::{
        orderbook::v1::{
            Order, OrderSide, OrderTimeInForce, OrderType, OrderMatch
        },
        transaction::v1::action::{
            CreateMarket, CreateOrder, CancelOrder, UpdateMarket
        },
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
        self, ActionRef, CheckedAction, CheckedActionExecutionError,
    },
    checked_actions::orderbook::CheckedActionError,
    component::Component,
    orderbook::state_ext::{MarketParams, OrderbookError, StateReadExt, StateWriteExt},
};

/// The order book component for the Astria sequencer.
#[derive(Debug, Default)]
pub struct OrderbookComponent;

/// A trait for executing checked actions in the orderbook component.
pub trait ExecuteOrderbookAction {
    fn execute<S: StateRead>(&self, component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError>;
}

#[async_trait::async_trait]
impl Component for OrderbookComponent {
    type AppState = ();

    /// Initialize the order book component at genesis.
    async fn init_chain<S: StateWrite>(_state: S, _app_state: &Self::AppState) -> astria_eyre::eyre::Result<()> {
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
    Other(#[from] astria_eyre::eyre::Report),
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

// We're already importing these from astria_core::protocol::transaction::v1::action
// No need to redefine them

impl ExecuteOrderbookAction for CheckedCreateOrder {
    fn execute<S: StateRead>(&self, component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        debug!(?self, "Executing CheckedCreateOrder");
        
        // This is a stub implementation - the actual implementation would:
        // 1. Check if market exists
        // 2. Validate market parameters 
        // 3. Create a new order ID
        // 4. Create the order
        // 5. Add order to the order book
        // 6. Match the order with existing orders
        
        // For now, we just log the action and return success
        info!(
            market = self.market,
            side = ?self.side,
            price = self.price,
            quantity = self.quantity,
            "Created order"
        );
        
        Ok(())
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

impl ExecuteOrderbookAction for CheckedCancelOrder {
    fn execute<S: StateRead>(&self, component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        debug!(?self, "Executing CheckedCancelOrder");
        
        // This is a stub implementation - the actual implementation would:
        // 1. Get the order
        // 2. Verify the sender is the owner
        // 3. Remove the order from the order book
        
        // For now, we just log the action and return success
        info!(
            order_id = self.order_id,
            "Cancelled order"
        );
        
        Ok(())
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

impl ExecuteOrderbookAction for CheckedCreateMarket {
    fn execute<S: StateRead>(&self, component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        debug!(?self, "Executing CheckedCreateMarket");
        
        // This is a stub implementation - the actual implementation would:
        // 1. Check if market already exists
        // 2. Create market parameters
        // 3. Add market to the order book
        
        // For now, we just log the action and return success
        info!(
            market = self.market,
            base_asset = self.base_asset,
            quote_asset = self.quote_asset,
            "Created market"
        );
        
        Ok(())
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

impl ExecuteOrderbookAction for CheckedUpdateMarket {
    fn execute<S: StateRead>(&self, component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        debug!(?self, "Executing CheckedUpdateMarket");
        
        // This is a stub implementation - the actual implementation would:
        // 1. Check if market exists
        // 2. Get current market parameters
        // 3. Update market parameters
        // 4. Update market parameters in state
        
        // For now, we just log the action and return success
        info!(
            market = self.market,
            paused = self.paused,
            "Updated market"
        );
        
        Ok(())
    }
}

