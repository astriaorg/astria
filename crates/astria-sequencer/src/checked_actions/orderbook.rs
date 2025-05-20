use astria_core::protocol::transaction::v1::action::{
    CancelOrder, CreateMarket, CreateOrder, UpdateMarket,
};
use std::sync::Arc;
use thiserror::Error;
use cnidarium::StateRead;

use crate::orderbook::{
    component::{CheckedCreateMarket, CheckedCreateOrder, CheckedCancelOrder, ExecuteOrderbookAction},
    OrderbookComponent, StateReadExt, utils,
};
// Use our own simplified ExecutionState instead of app's private one
struct ExecutionState<'a, S: StateRead> {
    state: &'a S
}

impl<'a, S: StateRead> ExecutionState<'a, S> {
    fn new(state: &'a S) -> Self {
        Self { state }
    }

    fn market_exists(&self, market_id: &str) -> bool {
        // This is a simplified implementation
        self.state.get_market_params(market_id).is_some()
    }

    fn get_order(&self, order_id: &str) -> Option<crate::orderbook::Order> {
        // This is a simplified implementation
        if let Some(proto_order) = self.state.get_order(order_id) {
            // Convert from protocol Order to our local Order
            Some(crate::orderbook::compat::order_from_proto(&proto_order))
        } else {
            None
        }
    }
}
use crate::checked_actions::{
    ActionRef, CheckedActionExecutionError,
};

/// Error type for checked orderbook actions
#[derive(Debug, Error)]
pub enum CheckedActionError {
    #[error(transparent)]
    OrderbookError(#[from] OrderbookError),
    #[error("Invalid address")]
    InvalidAddress,
    #[error("Internal error: {0}")]
    Internal(String),
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

    #[error("Order book operation failed: {0}")]
    OperationFailed(String),
}

/// Extension trait for OrderbookComponent to check CreateOrder actions
pub trait CheckCreateOrder {
    fn check_create_order<S: StateRead>(
        &self,
        state: &S,
        action: &CreateOrder,
        sender: String,
    ) -> Result<CheckedCreateOrder, CheckedActionError>;
}

impl CheckCreateOrder for OrderbookComponent {
    fn check_create_order<S: StateRead>(
        &self,
        state: &S,
        action: &CreateOrder,
        sender: String,
    ) -> Result<CheckedCreateOrder, CheckedActionError> {
        let execution_state = ExecutionState::new(state);

        // Check that the market exists
        if !execution_state.market_exists(&action.market) {
            return Err(CheckedActionError::from(OrderbookError::MarketNotFound(
                format!("Market {} not found", action.market),
            )));
        }

        // Validate order parameters
        if action.quantity.is_none() || action.quantity == Some(0) {
            return Err(CheckedActionError::from(OrderbookError::InvalidOrderParameters(
                "Quantity must be greater than 0".to_string(),
            )));
        }

        // Parse address
        let address = sender.parse().map_err(|_| {
            CheckedActionError::from(OrderbookError::InvalidOrderParameters(
                "Invalid sender address".to_string(),
            ))
        })?;

        // Convert fee_asset to string
        let fee_asset = action.fee_asset.to_string();
        
        // First get the protocol enums using the from_i32 methods
        let proto_side = utils::order_side_from_i32(action.side.into());
        let proto_type = utils::order_type_from_i32(action.r#type.into());
        let proto_tif = utils::time_in_force_from_i32(action.time_in_force.into());
        
        // Then convert to our local enums
        let side = utils::order_side_from_proto(proto_side);
        let order_type = utils::order_type_from_proto(proto_type);
        let time_in_force = utils::time_in_force_from_proto(proto_tif);

        Ok(CheckedCreateOrder {
            sender: address,
            market: action.market.clone(),
            side,
            order_type,
            price: match &action.price {
                Some(val) => val.to_string(),
                None => "0".to_string(),
            },
            quantity: match &action.quantity {
                Some(val) => val.to_string(),
                None => "0".to_string(),
            },
            time_in_force,
            fee_asset,
        })
    }
}

/// Extension trait for OrderbookComponent to check CancelOrder actions
pub trait CheckCancelOrder {
    fn check_cancel_order<S: StateRead>(
        &self,
        state: &S,
        action: &CancelOrder,
        sender: String,
    ) -> Result<CheckedCancelOrder, CheckedActionError>;
}

impl CheckCancelOrder for OrderbookComponent {
    fn check_cancel_order<S: StateRead>(
        &self,
        state: &S,
        action: &CancelOrder,
        sender: String,
    ) -> Result<CheckedCancelOrder, CheckedActionError> {
        let execution_state = ExecutionState::new(state);

        // Check that the order exists
        let order = execution_state
            .get_order(&action.order_id)
            .ok_or_else(|| {
                CheckedActionError::from(OrderbookError::OrderNotFound(
                    format!("Order {} not found", action.order_id),
                ))
            })?;

        // Check that the sender is the owner
        if order.owner != sender {
            return Err(CheckedActionError::from(OrderbookError::InvalidOrderParameters(
                "Only the owner can cancel their order".to_string(),
            )));
        }
        
        // Parse address
        let address = sender.parse().map_err(|_| {
            CheckedActionError::from(OrderbookError::InvalidOrderParameters(
                "Invalid sender address".to_string(),
            ))
        })?;
        
        // Convert fee_asset to string
        let fee_asset = action.fee_asset.to_string();

        Ok(CheckedCancelOrder {
            sender: address,
            order_id: action.order_id.clone(),
            fee_asset,
        })
    }
}

/// Extension trait for OrderbookComponent to check CreateMarket actions
pub trait CheckCreateMarket {
    fn check_create_market<S: StateRead>(
        &self,
        state: &S,
        action: &CreateMarket,
        sender: String,
    ) -> Result<CheckedCreateMarket, CheckedActionError>;
}

impl CheckCreateMarket for OrderbookComponent {
    fn check_create_market<S: StateRead>(
        &self,
        state: &S,
        action: &CreateMarket,
        sender: String,
    ) -> Result<CheckedCreateMarket, CheckedActionError> {
        let execution_state = ExecutionState::new(state);

        // Check that the market doesn't already exist
        if execution_state.market_exists(&action.market) {
            return Err(CheckedActionError::from(OrderbookError::MarketAlreadyExists(
                format!("Market {} already exists", action.market),
            )));
        }

        // Validate market parameters
        if action.base_asset.is_empty() || action.quote_asset.is_empty() {
            return Err(CheckedActionError::from(OrderbookError::InvalidOrderParameters(
                "Base and quote assets must be specified".to_string(),
            )));
        }
        
        // Parse address
        let address = sender.parse().map_err(|_| {
            CheckedActionError::from(OrderbookError::InvalidOrderParameters(
                "Invalid sender address".to_string(),
            ))
        })?;
        
        // Convert tick_size to string
        let tick_size = match action.tick_size {
            Some(val) => val.to_string(),
            None => "0".to_string(),
        };
        
        // Convert lot_size to string
        let lot_size = match action.lot_size {
            Some(val) => val.to_string(),
            None => "0".to_string(),
        };
        
        // Convert fee_asset to string
        let fee_asset = action.fee_asset.to_string();

        Ok(CheckedCreateMarket {
            sender: address,
            market: action.market.clone(),
            base_asset: action.base_asset.clone(),
            quote_asset: action.quote_asset.clone(),
            tick_size,
            lot_size,
            fee_asset,
        })
    }
}

/// Extension trait for OrderbookComponent to check UpdateMarket actions
pub trait CheckUpdateMarket {
    fn check_update_market<S: StateRead>(
        &self,
        state: &S,
        action: &UpdateMarket,
        sender: String,
    ) -> Result<crate::orderbook::component::CheckedUpdateMarket, CheckedActionError>;
}

impl CheckUpdateMarket for OrderbookComponent {
    fn check_update_market<S: StateRead>(
        &self,
        state: &S,
        action: &UpdateMarket,
        sender: String,
    ) -> Result<crate::orderbook::component::CheckedUpdateMarket, CheckedActionError> {
        let execution_state = ExecutionState::new(state);

        // Check that the market exists
        if !execution_state.market_exists(&action.market) {
            return Err(CheckedActionError::from(OrderbookError::MarketNotFound(
                format!("Market {} not found", action.market),
            )));
        }
        
        // Parse address
        let address = sender.parse().map_err(|_| {
            CheckedActionError::from(OrderbookError::InvalidOrderParameters(
                "Invalid sender address".to_string(),
            ))
        })?;
        
        // Convert tick_size to Option<String>
        let tick_size = match action.tick_size {
            Some(val) => Some(val.to_string()),
            None => None,
        };
        
        // Convert lot_size to Option<String>
        let lot_size = match action.lot_size {
            Some(val) => Some(val.to_string()),
            None => None,
        };
        
        // Convert fee_asset to string
        let fee_asset = action.fee_asset.to_string();

        Ok(crate::orderbook::component::CheckedUpdateMarket {
            sender: address,
            market: action.market.clone(),
            tick_size,
            lot_size,
            paused: action.paused,
            fee_asset,
        })
    }
}

// We need to implement ExecuteOrderbookAction as a wrapper around our action references
// to handle the borrow semantics correctly
impl<'a> ExecuteOrderbookAction for &'a CheckedCreateOrder {
    fn execute<S: StateRead>(&self, component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        // Just forward to the implementation on CheckedCreateOrder
        (*self).execute(component, state)
    }
}

impl<'a> ExecuteOrderbookAction for &'a CheckedCancelOrder {
    fn execute<S: StateRead>(&self, component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        // Just forward to the implementation on CheckedCancelOrder
        (*self).execute(component, state)
    }
}

impl<'a> ExecuteOrderbookAction for &'a CheckedCreateMarket {
    fn execute<S: StateRead>(&self, component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        // Just forward to the implementation on CheckedCreateMarket
        (*self).execute(component, state)
    }
}

impl<'a> ExecuteOrderbookAction for &'a crate::orderbook::component::CheckedUpdateMarket {
    fn execute<S: StateRead>(&self, component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        // Just forward to the implementation on CheckedUpdateMarket
        (*self).execute(component, state)
    }
}

// Add the orderbook-specific action references to ActionRef
impl<'a> ActionRef<'a> {
    pub fn apply_orderbook<S: StateRead>(
        &self,
        _component: &OrderbookComponent,
        state: &mut S,
    ) -> Result<(), CheckedActionExecutionError> {
        // Create a new OrderbookComponent (it's small and has no state) and wrap it in Arc
        let component_arc = Arc::new(OrderbookComponent::default());
        
        match self {
            ActionRef::OrderbookCreateOrder(action) => {
                // Now we can just use ExecuteOrderbookAction directly on the reference
                action.execute(component_arc.clone(), state)
            }
            ActionRef::OrderbookCancelOrder(action) => {
                // Now we can just use ExecuteOrderbookAction directly on the reference
                action.execute(component_arc.clone(), state)
            }
            ActionRef::OrderbookCreateMarket(action) => {
                // Now we can just use ExecuteOrderbookAction directly on the reference
                action.execute(component_arc.clone(), state)
            }
            ActionRef::OrderbookUpdateMarket(action) => {
                // Now we can just use ExecuteOrderbookAction directly on the reference
                action.execute(component_arc.clone(), state)
            }
            _ => Ok(()),
        }
    }
}