use std::sync::Arc;

use astria_core::{
    primitive::v1::Address,
};
use astria_eyre::eyre;
use cnidarium::{StateRead, StateWrite};
use tendermint::abci;
use thiserror::Error;
use tracing::{debug, info};

use crate::{
    checked_actions::{CheckedActionExecutionError, CheckedActionFeeError},
    checked_actions::orderbook::CheckedActionError,
    component::Component,
    orderbook::state_ext::OrderbookError,
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
    async fn init_chain<S: StateWrite>(mut state: S, _app_state: &Self::AppState) -> astria_eyre::eyre::Result<()> {
        info!("Initializing OrderbookComponent");
        
        // Insert a test market for development purposes
        match crate::orderbook::debug::force_insert_test_market(&mut state) {
            Ok(_) => {
                info!("Successfully inserted test market during initialization");
                // Verify storage
                crate::orderbook::debug::debug_check_market_data(&state);
            },
            Err(e) => {
                info!(error = e, "Failed to insert test market during initialization");
            }
        }
        
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

/// Order side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    /// Buy side (bids)
    Buy,
    /// Sell side (asks)
    Sell,
}

/// Order type (limit, market, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    /// Limit order
    Limit,
    /// Market order
    Market,
}

/// Order time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderTimeInForce {
    /// Good till cancelled
    GoodTillCancelled,
    /// Fill or kill
    FillOrKill,
    /// Immediate or cancel
    ImmediateOrCancel,
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

impl ExecuteOrderbookAction for CheckedCreateOrder {
    fn execute<S: StateRead>(&self, _component: Arc<OrderbookComponent>, _state: &mut S) -> Result<(), CheckedActionExecutionError> {
        debug!(?self, "Executing CheckedCreateOrder");
        
        // Parse inputs to verify they're valid
        let _price = self.price.parse::<u128>()
            .map_err(|_| {
                // Invalid price
                CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled { 
                        action_name: "create_order" 
                    }
                )
            })?;
            
        let _quantity = self.quantity.parse::<u128>()
            .map_err(|_| {
                // Invalid quantity
                CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled { 
                        action_name: "create_order" 
                    }
                )
            })?;
            
        // For now, just log the order creation without modifying state
        // In a proper implementation, we would use state.put_order() from StateWriteExt
        info!(
            market = self.market,
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
    fn execute<S: StateRead>(&self, _component: Arc<OrderbookComponent>, _state: &mut S) -> Result<(), CheckedActionExecutionError> {
        debug!(?self, "Executing CheckedCancelOrder");
                
        // For now, just log the order cancellation without modifying state
        // In a proper implementation, we would use state.remove_order() from StateWriteExt
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
    fn execute<S: StateRead>(&self, _component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        debug!(?self, "Executing CheckedCreateMarket");
        
        // Parse tick_size and lot_size to u128 to validate them
        let tick_size = self.tick_size.parse::<u128>()
            .map_err(|_| {
                // Invalid tick size
                CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled { 
                        action_name: "create_market" 
                    }
                )
            })?;
            
        let lot_size = self.lot_size.parse::<u128>()
            .map_err(|_| {
                // Invalid lot size
                CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled { 
                        action_name: "create_market" 
                    }
                )
            })?;
        
        // Create market parameters
        let market_params = crate::orderbook::state_ext::MarketParams {
            base_asset: self.base_asset.clone(),
            quote_asset: self.quote_asset.clone(),
            tick_size: Some(tick_size),
            lot_size: Some(lot_size),
            paused: false,
        };
        
        // Log market creation - would actually store in state with a more complete implementation
        // For now we'll just log information without actually modifying state
        info!(
            market = self.market,
            base_asset = self.base_asset,
            quote_asset = self.quote_asset,
            tick_size,
            lot_size,
            "Created market (but not storing in state yet)"
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
    fn execute<S: StateRead>(&self, _component: Arc<OrderbookComponent>, _state: &mut S) -> Result<(), CheckedActionExecutionError> {
        debug!(?self, "Executing CheckedUpdateMarket");
        
        // Parse tick_size if provided (for validation)
        if let Some(tick_size_str) = &self.tick_size {
            let _tick_size = tick_size_str.parse::<u128>()
                .map_err(|_| {
                    // Invalid tick size
                    CheckedActionExecutionError::Fee(
                        CheckedActionFeeError::ActionDisabled { 
                            action_name: "update_market" 
                        }
                    )
                })?;
        }
        
        // Parse lot_size if provided (for validation)
        if let Some(lot_size_str) = &self.lot_size {
            let _lot_size = lot_size_str.parse::<u128>()
                .map_err(|_| {
                    // Invalid lot size
                    CheckedActionExecutionError::Fee(
                        CheckedActionFeeError::ActionDisabled { 
                            action_name: "update_market" 
                        }
                    )
                })?;
        }
        
        // For a proper implementation, we would:
        // 1. Get existing market params with state.get_market_params from StateReadExt
        // 2. Update the parameters
        // 3. Write back with state.update_market_params from StateWriteExt
        // For now, just log the update without modifying state
        info!(
            market = self.market,
            tick_size = ?self.tick_size,
            lot_size = ?self.lot_size,
            paused = self.paused,
            "Updated market"
        );
        
        Ok(())
    }
}