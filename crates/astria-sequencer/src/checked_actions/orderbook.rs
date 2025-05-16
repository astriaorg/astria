use astria_core::protocol::orderbook::v1::{
    CreateMarket, CreateOrder, CancelOrder, OrderMatch,
};
use std::sync::Arc;
use thiserror::Error;

use crate::orderbook::{
    component::{CheckedCreateMarket, CheckedCreateOrder, CheckedCancelOrder},
    OrderbookComponent, StateReadExt, StateWriteExt,
};
use crate::app::execution_state::ExecutionState;
use crate::component::Component;
use crate::checked_actions::{
    ActionRef, CheckedAction, CheckedActionError, CheckedActionExecutionError,
};

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
    fn check_create_order(
        &self,
        execution_state: &ExecutionState,
        action: &CreateOrder,
        sender: String,
    ) -> Result<CheckedCreateOrder, CheckedActionError>;
}

impl CheckCreateOrder for OrderbookComponent {
    fn check_create_order(
        &self,
        execution_state: &ExecutionState,
        action: &CreateOrder,
        sender: String,
    ) -> Result<CheckedCreateOrder, CheckedActionError> {
        // Check that the market exists
        if !execution_state.market_exists(&action.market) {
            return Err(CheckedActionError::from(OrderbookError::MarketNotFound(
                format!("Market {} not found", action.market),
            )));
        }

        // Validate order parameters
        if action.quantity.is_empty() || action.quantity == "0" {
            return Err(CheckedActionError::from(OrderbookError::InvalidOrderParameters(
                "Quantity must be greater than 0".to_string(),
            )));
        }

        Ok(CheckedCreateOrder {
            sender: sender.parse().map_err(|_| {
                CheckedActionError::from(OrderbookError::InvalidOrderParameters(
                    "Invalid sender address".to_string(),
                ))
            })?,
            market: action.market.clone(),
            side: action.side(),
            order_type: action.type_(),
            price: action.price.clone(),
            quantity: action.quantity.clone(),
            time_in_force: action.time_in_force(),
            fee_asset: action.fee_asset.clone(),
        })
    }
}

/// Extension trait for OrderbookComponent to check CancelOrder actions
pub trait CheckCancelOrder {
    fn check_cancel_order(
        &self,
        execution_state: &ExecutionState,
        action: &CancelOrder,
        sender: String,
    ) -> Result<CheckedCancelOrder, CheckedActionError>;
}

impl CheckCancelOrder for OrderbookComponent {
    fn check_cancel_order(
        &self,
        execution_state: &ExecutionState,
        action: &CancelOrder,
        sender: String,
    ) -> Result<CheckedCancelOrder, CheckedActionError> {
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

        Ok(CheckedCancelOrder {
            sender: sender.parse().map_err(|_| {
                CheckedActionError::from(OrderbookError::InvalidOrderParameters(
                    "Invalid sender address".to_string(),
                ))
            })?,
            order_id: action.order_id.clone(),
            fee_asset: action.fee_asset.clone(),
        })
    }
}

/// Extension trait for OrderbookComponent to check CreateMarket actions
pub trait CheckCreateMarket {
    fn check_create_market(
        &self,
        execution_state: &ExecutionState,
        action: &CreateMarket,
        sender: String,
    ) -> Result<CheckedCreateMarket, CheckedActionError>;
}

impl CheckCreateMarket for OrderbookComponent {
    fn check_create_market(
        &self,
        execution_state: &ExecutionState,
        action: &CreateMarket,
        sender: String,
    ) -> Result<CheckedCreateMarket, CheckedActionError> {
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

        Ok(CheckedCreateMarket {
            sender: sender.parse().map_err(|_| {
                CheckedActionError::from(OrderbookError::InvalidOrderParameters(
                    "Invalid sender address".to_string(),
                ))
            })?,
            market: action.market.clone(),
            base_asset: action.base_asset.clone(),
            quote_asset: action.quote_asset.clone(),
            tick_size: action.tick_size.clone(),
            lot_size: action.lot_size.clone(),
            fee_asset: action.fee_asset.clone(),
        })
    }
}

// Add the orderbook-specific action references to ActionRef
impl ActionRef {
    pub fn apply(
        &self,
        component: &OrderbookComponent,
        execution_state: &mut ExecutionState,
    ) -> Result<(), CheckedActionExecutionError> {
        match self {
            ActionRef::OrderbookCreateOrder(action) => {
                let component = Arc::new(component.clone());
                action.execute(component, execution_state)
            }
            ActionRef::OrderbookCancelOrder(action) => {
                let component = Arc::new(component.clone());
                action.execute(component, execution_state)
            }
            ActionRef::OrderbookCreateMarket(action) => {
                let component = Arc::new(component.clone());
                action.execute(component, execution_state)
            }
            _ => Ok(()),
        }
    }
}