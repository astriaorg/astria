use std::sync::Arc;

use astria_core::{
    primitive::v1::Address,
};
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
    fn execute<S: StateRead + cnidarium::StateWrite>(&self, component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError>;
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
    /// 
    /// This method is called at the beginning of each block and can be used
    /// to prepare the orderbook for the incoming transactions.
    async fn begin_block<S: StateWrite + 'static>(
        _state: &mut Arc<S>,
        begin_block: &abci::request::BeginBlock,
    ) -> astria_eyre::eyre::Result<()> {
        info!(
            height = ?begin_block.header.height,
            "OrderbookComponent: begin_block"
        );
        
        // In a more advanced implementation, we might:
        // 1. Check for expired orders and remove them
        // 2. Initialize block-level statistics for the orderbook
        // 3. Apply any market status changes scheduled for this block
        
        Ok(())
    }

    /// Process end_block events for the order book.
    /// 
    /// This method is called at the end of each block after all transactions
    /// have been processed. It can be used to finalize any pending operations,
    /// update statistics, or handle global order matching.
    async fn end_block<S: StateWrite + 'static>(
        _state: &mut Arc<S>,
        end_block: &abci::request::EndBlock,
    ) -> astria_eyre::eyre::Result<()> {
        info!(
            height = ?end_block.height,
            "OrderbookComponent: end_block"
        );
        
        // Create a matching engine - commented out because currently unused
        // let _matching_engine = crate::orderbook::matching_engine::MatchingEngine::default();
        
        // In a production implementation, we would:
        // 1. Process any buffered orders that haven't been matched yet
        // 2. Record trading statistics for the block
        // 3. Update market prices and indices
        // 4. Emit events for orderbook activity during the block
        
        // Note: The actual order matching happens in the transaction processing
        // rather than in end_block, as orders should be matched as they come in.
        // However, we could also implement a periodic batch auction model where
        // orders are collected during the block and matched at end_block.
        
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
    fn execute<S: StateRead + cnidarium::StateWrite>(&self, _component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        use crate::orderbook::state_ext::{StateWriteExt, StateReadExt};
        use uuid::Uuid;
        
        debug!(?self, "Executing CheckedCreateOrder");
        
        // Parse numeric values
        let price = self.price.parse::<u128>()
            .map_err(|_| {
                // Invalid price
                CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled { 
                        action_name: "create_order" 
                    }
                )
            })?;
            
        let quantity = self.quantity.parse::<u128>()
            .map_err(|_| {
                // Invalid quantity
                CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled { 
                        action_name: "create_order" 
                    }
                )
            })?;
        
        // Check that the market exists
        if !state.market_exists(&self.market) {
            return Err(CheckedActionExecutionError::Fee(
                CheckedActionFeeError::ActionDisabled {
                    action_name: "create_order",
                }
            ));
        }
        
        // Check market parameters for tick size and lot size
        if let Some(market_params) = state.get_market_params(&self.market) {
            // Check if the market is paused
            if market_params.paused {
                return Err(CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled {
                        action_name: "create_order",
                    }
                ));
            }
            
            // Check tick size
            if let Some(tick_size) = market_params.tick_size {
                if price % tick_size != 0 {
                    return Err(CheckedActionExecutionError::Fee(
                        CheckedActionFeeError::ActionDisabled {
                            action_name: "create_order",
                        }
                    ));
                }
            }
            
            // Check lot size
            if let Some(lot_size) = market_params.lot_size {
                if quantity % lot_size != 0 {
                    return Err(CheckedActionExecutionError::Fee(
                        CheckedActionFeeError::ActionDisabled {
                            action_name: "create_order",
                        }
                    ));
                }
            }
        }
        
        // Convert our local enums to protocol enums
        let proto_side = match self.side {
            OrderSide::Buy => astria_core::protocol::orderbook::v1::OrderSide::Buy as i32,
            OrderSide::Sell => astria_core::protocol::orderbook::v1::OrderSide::Sell as i32,
        };
        
        let proto_type = match self.order_type {
            OrderType::Limit => astria_core::protocol::orderbook::v1::OrderType::Limit as i32,
            OrderType::Market => astria_core::protocol::orderbook::v1::OrderType::Market as i32,
        };
        
        let proto_tif = match self.time_in_force {
            OrderTimeInForce::GoodTillCancelled => astria_core::protocol::orderbook::v1::OrderTimeInForce::Gtc as i32,
            OrderTimeInForce::ImmediateOrCancel => astria_core::protocol::orderbook::v1::OrderTimeInForce::Ioc as i32,
            OrderTimeInForce::FillOrKill => astria_core::protocol::orderbook::v1::OrderTimeInForce::Fok as i32,
        };
        
        // Convert numeric values to protocol Uint128 format
        let price_opt = crate::orderbook::string_to_uint128_option(&self.price);
        let quantity_opt = crate::orderbook::string_to_uint128_option(&self.quantity);
        
        // Create address format
        let owner = Some(astria_core::generated::astria::primitive::v1::Address {
            bech32m: self.sender.to_string(),
        });
        
        // Generate a unique order ID
        let order_id = Uuid::new_v4().to_string();
        
        // Create the order
        let order = astria_core::protocol::orderbook::v1::Order {
            id: order_id,
            owner,
            market: self.market.clone(),
            side: proto_side,
            r#type: proto_type,
            price: price_opt,
            quantity: quantity_opt.clone(),
            remaining_quantity: quantity_opt,
            created_at: chrono::Utc::now().timestamp() as u64,
            time_in_force: proto_tif,
            fee_asset: self.fee_asset.clone(),
        };
        
        // Create a matching engine
        let matching_engine = crate::orderbook::matching_engine::MatchingEngine::default();
        
        // Process the order through the matching engine
        let matches = matching_engine.process_order(state, order.clone())
            .map_err(|e| {
                CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled {
                        action_name: "create_order",
                    }
                )
            })?;
        
        // Record any trades that occurred
        for trade_match in matches {
            state.record_trade(trade_match)
                .map_err(|e| {
                    CheckedActionExecutionError::Fee(
                        CheckedActionFeeError::ActionDisabled {
                            action_name: "create_order",
                        }
                    )
                })?;
        }
        
        info!(
            order_id = order.id,
            market = self.market,
            side = ?self.side,
            price = self.price,
            quantity = self.quantity,
            "Created and processed order"
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
    fn execute<S: StateRead + cnidarium::StateWrite>(&self, _component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        use crate::orderbook::state_ext::{StateWriteExt, StateReadExt};
        
        debug!(?self, "Executing CheckedCancelOrder");
        
        // Check that the order exists
        let order = match state.get_order(&self.order_id) {
            Some(order) => order,
            None => {
                return Err(CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled {
                        action_name: "cancel_order",
                    }
                ));
            }
        };
        
        // Check that the sender is the owner of the order
        if let Some(owner) = &order.owner {
            if owner.bech32m != self.sender.to_string() {
                return Err(CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled {
                        action_name: "cancel_order",
                    }
                ));
            }
        }
        
        // Cancel the order
        state.remove_order(&self.order_id)
            .map_err(|e| {
                CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled {
                        action_name: "cancel_order",
                    }
                )
            })?;
        
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
    fn execute<S: StateRead + cnidarium::StateWrite>(&self, _component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        use crate::orderbook::state_ext::{StateWriteExt, StateReadExt};
        
        // Add very prominent logging for create market actions
        tracing::warn!("🔷🔷🔷 CREATE MARKET ACTION RECEIVED 🔷🔷🔷");
        tracing::warn!(
            market = %self.market,
            base_asset = %self.base_asset,
            quote_asset = %self.quote_asset,
            tick_size = %self.tick_size,
            lot_size = %self.lot_size,
            "📊 MARKET CREATION PARAMETERS 📊"
        );
        
        // Validate market name format
        if self.market.is_empty() {
            tracing::error!("❌ Empty market name in create market action");
            return Err(CheckedActionExecutionError::Fee(
                CheckedActionFeeError::ActionDisabled { 
                    action_name: "create_market" 
                }
            ));
        }
        
        // Optionally validate market name format (example: should be BASE/QUOTE format)
        if !self.market.contains('/') {
            tracing::warn!("⚠️ Market name does not follow BASE/QUOTE format: {}", self.market);
            // Could return an error, but let's just warn for now to be flexible
        }
        
        // Validate asset names aren't empty
        if self.base_asset.is_empty() || self.quote_asset.is_empty() {
            tracing::error!("❌ Empty asset name in create market action");
            return Err(CheckedActionExecutionError::Fee(
                CheckedActionFeeError::ActionDisabled { 
                    action_name: "create_market" 
                }
            ));
        }
        
        // Check if the market already exists - better to explicitly check and return custom error
        if state.market_exists(&self.market) {
            tracing::error!("❌ Market already exists: {}", self.market);
            return Err(CheckedActionExecutionError::Fee(
                CheckedActionFeeError::ActionDisabled { 
                    action_name: "create_market" 
                }
            ));
        }
        
        // Parse tick_size and lot_size to u128 to validate them
        let tick_size = self.tick_size.parse::<u128>()
            .map_err(|_| {
                // Invalid tick size
                tracing::error!("❌ Invalid tick size in create market action: {}", self.tick_size);
                CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled { 
                        action_name: "create_market" 
                    }
                )
            })?;
            
        let lot_size = self.lot_size.parse::<u128>()
            .map_err(|_| {
                // Invalid lot size
                tracing::error!("❌ Invalid lot size in create market action: {}", self.lot_size);
                CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled { 
                        action_name: "create_market" 
                    }
                )
            })?;
        
        // Additional validations for tick_size and lot_size
        if tick_size == 0 {
            tracing::error!("❌ Tick size must be greater than zero");
            return Err(CheckedActionExecutionError::Fee(
                CheckedActionFeeError::ActionDisabled { 
                    action_name: "create_market" 
                }
            ));
        }
        
        if lot_size == 0 {
            tracing::error!("❌ Lot size must be greater than zero");
            return Err(CheckedActionExecutionError::Fee(
                CheckedActionFeeError::ActionDisabled { 
                    action_name: "create_market" 
                }
            ));
        }
        
        // Create market parameters
        let market_params = crate::orderbook::state_ext::MarketParams {
            base_asset: self.base_asset.clone(),
            quote_asset: self.quote_asset.clone(),
            tick_size: Some(tick_size),
            lot_size: Some(lot_size),
            paused: false, // New markets are active by default
        };
        
        // Now actually try to store the market
        match state.add_market(&self.market, market_params.clone()) {
            Ok(_) => {
                tracing::warn!(
                    "✅ MARKET SUCCESSFULLY CREATED AND STORED: {}",
                    self.market
                );
                
                // Log the market parameters that were stored
                tracing::info!(
                    market = %self.market,
                    base_asset = %market_params.base_asset,
                    quote_asset = %market_params.quote_asset,
                    tick_size = ?market_params.tick_size,
                    lot_size = ?market_params.lot_size,
                    paused = market_params.paused,
                    "Market parameters stored"
                );
                
                // Verify storage immediately
                crate::orderbook::debug::debug_check_market_data(state);
                
                // Verify the market was added to the list of all markets
                let all_markets = state.get_markets();
                if all_markets.contains(&self.market) {
                    tracing::info!("✅ Market added to the list of all markets");
                } else {
                    tracing::warn!("⚠️ Market not found in the list of all markets");
                }
            },
            Err(err) => {
                tracing::error!(
                    error = ?err,
                    "❌ FAILED TO CREATE MARKET: {}",
                    self.market
                );
                
                return Err(CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled { 
                        action_name: "create_market" 
                    }
                ));
            }
        }
        
        info!(
            market = self.market,
            base_asset = self.base_asset,
            quote_asset = self.quote_asset,
            tick_size = self.tick_size,
            lot_size = self.lot_size,
            "Successfully created new market"
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
    fn execute<S: StateRead + cnidarium::StateWrite>(&self, _component: Arc<OrderbookComponent>, state: &mut S) -> Result<(), CheckedActionExecutionError> {
        use crate::orderbook::state_ext::{StateWriteExt, StateReadExt};
        
        debug!(?self, "Executing CheckedUpdateMarket");
        
        // Add visible logging for market updates
        tracing::warn!("🔸🔸🔸 UPDATE MARKET ACTION RECEIVED 🔸🔸🔸");
        tracing::warn!(
            market = %self.market,
            tick_size = ?self.tick_size,
            lot_size = ?self.lot_size,
            paused = self.paused,
            "📊 MARKET UPDATE PARAMETERS 📊"
        );
        
        // Validations - parse numeric values (if provided) to ensure they're valid
        let tick_size_u128 = if let Some(tick_size_str) = &self.tick_size {
            let tick_size = tick_size_str.parse::<u128>()
                .map_err(|_| {
                    tracing::error!("❌ Invalid tick size in update market action: {}", tick_size_str);
                    CheckedActionExecutionError::Fee(
                        CheckedActionFeeError::ActionDisabled { 
                            action_name: "update_market" 
                        }
                    )
                })?;
            Some(tick_size)
        } else {
            None
        };
        
        let lot_size_u128 = if let Some(lot_size_str) = &self.lot_size {
            let lot_size = lot_size_str.parse::<u128>()
                .map_err(|_| {
                    tracing::error!("❌ Invalid lot size in update market action: {}", lot_size_str);
                    CheckedActionExecutionError::Fee(
                        CheckedActionFeeError::ActionDisabled { 
                            action_name: "update_market" 
                        }
                    )
                })?;
            Some(lot_size)
        } else {
            None
        };
        
        // Check that the market exists
        if !state.market_exists(&self.market) {
            tracing::error!("❌ Market not found in update market action: {}", self.market);
            return Err(CheckedActionExecutionError::Fee(
                CheckedActionFeeError::ActionDisabled {
                    action_name: "update_market",
                }
            ));
        }
        
        // Get existing market parameters
        let existing_params = match state.get_market_params(&self.market) {
            Some(params) => params,
            None => {
                tracing::error!("❌ Market parameters not found for existing market: {}", self.market);
                return Err(CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled {
                        action_name: "update_market",
                    }
                ));
            }
        };
        
        // Create updated market parameters by merging existing with new values
        let updated_params = crate::orderbook::state_ext::MarketParams {
            // Keep base and quote assets the same - these should not change
            base_asset: existing_params.base_asset,
            quote_asset: existing_params.quote_asset,
            // Update tick size if provided, otherwise keep existing
            tick_size: tick_size_u128.or(existing_params.tick_size),
            // Update lot size if provided, otherwise keep existing
            lot_size: lot_size_u128.or(existing_params.lot_size),
            // Always update paused status
            paused: self.paused,
        };
        
        // Update the market parameters in state
        match state.update_market_params(&self.market, updated_params.clone()) {
            Ok(_) => {
                tracing::warn!(
                    "✅ MARKET SUCCESSFULLY UPDATED: {}",
                    self.market
                );
                
                // Verify storage immediately by retrieving and logging
                if let Some(params) = state.get_market_params(&self.market) {
                    tracing::info!(
                        market = %self.market,
                        base_asset = %params.base_asset,
                        quote_asset = %params.quote_asset,
                        tick_size = ?params.tick_size,
                        lot_size = ?params.lot_size,
                        paused = params.paused,
                        "Updated market parameters retrieved from storage"
                    );
                }
            },
            Err(err) => {
                tracing::error!(
                    error = ?err,
                    "❌ FAILED TO UPDATE MARKET: {}",
                    self.market
                );
                
                return Err(CheckedActionExecutionError::Fee(
                    CheckedActionFeeError::ActionDisabled {
                        action_name: "update_market",
                    }
                ));
            }
        }
        
        info!(
            market = self.market,
            tick_size = ?self.tick_size,
            lot_size = ?self.lot_size,
            paused = self.paused,
            "Successfully updated market parameters"
        );
        
        Ok(())
    }
}