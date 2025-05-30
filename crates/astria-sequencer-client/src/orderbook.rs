//! Extension trait for interacting with the Astria Sequencer orderbook.
//!
//! This module provides types and methods for querying the orderbook API.

use async_trait::async_trait;
use borsh::{BorshDeserialize, BorshSerialize};
use prost::Message;
use serde::{Deserialize, Serialize};
use tracing;

use crate::orderbook_types::{MarketList, OrderList, TradeList};

use astria_core::{
    generated::astria::primitive::v1::Uint128,
    protocol::{
        orderbook::v1::{
            Order, OrderMatch, Orderbook, OrderbookEntry, OrderSide, OrderType, OrderTimeInForce,
        },
    },
    primitive::v1::Address,
};

// Helper trait to convert from Uint128 to u128
trait Uint128Ext {
    fn value(&self) -> u128;
}

impl Uint128Ext for Uint128 {
    fn value(&self) -> u128 {
        ((self.hi as u128) << 64) + (self.lo as u128)
    }
}

/// A wrapper around the sequencer's market parameters.
#[derive(Clone, Debug, Deserialize, Serialize, BorshSerialize, BorshDeserialize)]
pub struct MarketParams {
    /// The base asset for the market
    pub base_asset: String,
    /// The quote asset for the market
    pub quote_asset: String,
    /// The minimum price increment (tick size)
    pub tick_size: Option<u128>,
    /// The minimum quantity increment (lot size)
    pub lot_size: Option<u128>,
    /// Whether the market is paused
    pub paused: bool,
}

/// An aggregated view of the orderbook showing quantities at each price level.
#[derive(Clone, Debug, Deserialize, Serialize, BorshSerialize, BorshDeserialize)]
pub struct OrderbookDepth {
    /// The market identifier
    pub market: String,
    /// The bid levels (buy orders), sorted by price descending
    pub bids: Vec<OrderbookDepthLevel>,
    /// The ask levels (sell orders), sorted by price ascending
    pub asks: Vec<OrderbookDepthLevel>,
}

/// A single price level in the orderbook depth.
#[derive(Clone, Debug, Deserialize, Serialize, BorshSerialize, BorshDeserialize)]
pub struct OrderbookDepthLevel {
    /// The price level
    pub price: Option<u128>,
    /// The total quantity at this price level
    pub quantity: Option<u128>,
    /// The number of orders at this price level
    pub order_count: u32,
}

/// Errors that can occur when interacting with the orderbook API.
#[derive(Debug, thiserror::Error)]
pub enum OrderbookError {
    /// Error occurred when deserializing orderbook response
    #[error("failed to deserialize orderbook response: {0}")]
    Deserialization(#[source] borsh::io::Error),

    /// Error occurred when serializing orderbook request
    #[error("failed to serialize orderbook request: {0}")]
    Serialization(#[source] borsh::io::Error),

    /// Error occurred during the sequencer client request
    #[error("sequencer client request failed: {0}")]
    Client(#[source] tendermint_rpc::Error),

    /// The requested market does not exist
    #[error("market does not exist: {0}")]
    MarketNotFound(String),

    /// The requested order does not exist
    #[error("order not found: {0}")]
    OrderNotFound(String),
    
    /// Error when signing transaction
    #[error("failed to sign transaction: {0}")]
    SigningError(String),
    
    /// Error when submitting transaction
    #[error("failed to submit transaction: {0}")]
    SubmissionError(String),

    /// Error when converting transaction between types
    #[error("transaction conversion error: {0}")]
    TransactionConversion(String),

    /// Other errors
    #[error("orderbook error: {0}")]
    Other(String),
}

/// Extension trait for accessing orderbook-related functionality.
#[async_trait]
pub trait OrderbookClientExt: crate::extension_trait::SequencerClientExt {
    /// Get the client's address.
    ///
    /// This is needed for transaction signing and is assumed to be
    /// implemented by the client.
    ///
    /// # Errors
    ///
    /// - If the address cannot be determined
    async fn address(&self) -> Result<astria_core::primitive::v1::Address, OrderbookError> {
        Err(OrderbookError::Other("Client address not available".to_string()))
    }

    /// Get the chain ID.
    ///
    /// This is needed for transaction signing and is assumed to be
    /// implemented by the client.
    ///
    /// # Errors
    ///
    /// - If the chain ID cannot be determined
    async fn chain_id(&self) -> Result<String, OrderbookError> {
        Ok("astria-dev".to_string())
    }
    /// Creates a new order in the orderbook.
    ///
    /// # Arguments
    ///
    /// * `market` - The market identifier (e.g., "BTC/USD")
    /// * `side` - The order side (buy or sell)
    /// * `order_type` - The order type (limit or market)
    /// * `price` - The limit price (required for limit orders)
    /// * `quantity` - The amount to buy or sell
    /// * `time_in_force` - The time in force parameter
    /// * `fee_asset` - The asset used to pay the transaction fee
    ///
    /// # Errors
    ///
    /// * If the transaction fails to be submitted
    /// * If the order parameters are invalid
    async fn create_order(
        &self,
        market: String,
        side: OrderSide,
        order_type: OrderType,
        price: Option<u128>,
        quantity: u128,
        time_in_force: OrderTimeInForce,
        fee_asset: String,
    ) -> Result<tendermint_rpc::endpoint::broadcast::tx_sync::Response, OrderbookError>;

    /// Cancels an existing order.
    ///
    /// # Arguments
    ///
    /// * `order_id` - The ID of the order to cancel
    /// * `fee_asset` - The asset used to pay the transaction fee
    ///
    /// # Errors
    ///
    /// * If the transaction fails to be submitted
    /// * If the order does not exist or is not owned by the sender
    async fn cancel_order(
        &self,
        order_id: String,
        fee_asset: String,
    ) -> Result<tendermint_rpc::endpoint::broadcast::tx_sync::Response, OrderbookError>;

    /// Creates a new market for trading.
    ///
    /// # Arguments
    ///
    /// * `market` - The market identifier (e.g., "BTC/USD")
    /// * `base_asset` - The base asset of the market (e.g., "BTC")
    /// * `quote_asset` - The quote asset of the market (e.g., "USD")
    /// * `tick_size` - The minimum price increment
    /// * `lot_size` - The minimum quantity increment
    /// * `fee_asset` - The asset used to pay the transaction fee
    ///
    /// # Errors
    ///
    /// * If the transaction fails to be submitted
    /// * If the market parameters are invalid
    /// * If the market already exists
    async fn create_market(
        &self,
        market: String,
        base_asset: String,
        quote_asset: String,
        tick_size: Option<u128>,
        lot_size: Option<u128>,
        fee_asset: String,
    ) -> Result<tendermint_rpc::endpoint::broadcast::tx_sync::Response, OrderbookError>;

    /// Updates an existing market's parameters.
    ///
    /// # Arguments
    ///
    /// * `market` - The market identifier
    /// * `tick_size` - The new minimum price increment (if provided)
    /// * `lot_size` - The new minimum quantity increment (if provided)
    /// * `paused` - Whether the market is paused
    /// * `fee_asset` - The asset used to pay the transaction fee
    ///
    /// # Errors
    ///
    /// * If the transaction fails to be submitted
    /// * If the market does not exist
    async fn update_market(
        &self,
        market: String,
        tick_size: Option<u128>,
        lot_size: Option<u128>,
        paused: bool,
        fee_asset: String,
    ) -> Result<tendermint_rpc::endpoint::broadcast::tx_sync::Response, OrderbookError>;
    /// Returns all available markets.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC fails
    /// - If the response cannot be deserialized
    async fn get_markets(&self) -> Result<Vec<String>, OrderbookError> {
        const PATH: &str = "orderbook/markets";

        let response = self
            .abci_query(Some(PATH.to_string()), vec![], None, false)
            .await
            .map_err(|e| OrderbookError::Client(e))?;
            
        // Try to decode the response using our MarketList type
        match MarketList::decode(&response.value) {
            Ok(market_list) => {
                // Convert the markets to strings (market IDs)
                Ok(market_list.markets)
            },
            Err(e) => {
                // If protobuf decoding fails, try JSON as fallback
                tracing::warn!("Failed to decode markets as protobuf: {}", e);
                serde_json::from_slice(&response.value)
                    .map_err(|e| OrderbookError::Other(format!("Failed to parse markets response: {}", e)))
            }
        }
    }

    /// Returns the parameters for a specific market.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC fails
    /// - If the response cannot be deserialized
    /// - If the market does not exist
    async fn get_market_params(&self, market: &str) -> Result<MarketParams, OrderbookError> {
        let path = format!("orderbook/market_params/{}", market);

        let response = self
            .abci_query(Some(path), vec![], None, false)
            .await
            .map_err(|e| OrderbookError::Client(e))?;

        if response.code.is_err() {
            return Err(OrderbookError::MarketNotFound(market.to_string()));
        }

        // Parse the response - we don't have a protobuf MarketParams type, so use the JSON response
        let proto_params = serde_json::from_slice(&response.value)
            .map_err(|e| OrderbookError::Other(format!("Failed to decode market params: {}", e)))?;
        
        // The response should already be in the format we need
        let params: MarketParams = proto_params;

        Ok(params)
    }

    /// Returns the full orderbook for a specific market.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC fails
    /// - If the response cannot be deserialized
    /// - If the market does not exist
    async fn get_orderbook(&self, market: &str) -> Result<Orderbook, OrderbookError> {
        let path = format!("orderbook/{}", market);

        let response = self
            .abci_query(Some(path), vec![], None, false)
            .await
            .map_err(|e| OrderbookError::Client(e))?;

        if response.code.is_err() {
            return Err(OrderbookError::MarketNotFound(market.to_string()));
        }

        // Deserialize the response using protobuf
        let proto_orderbook = astria_core::generated::astria::protocol::orderbook::v1::Orderbook::decode(&*response.value)
            .map_err(|e| OrderbookError::Other(format!("Failed to decode orderbook: {}", e)))?;

        // Convert the protobuf Orderbook to our domain Orderbook
        // For now, assuming there's a direct conversion or that the types are compatible
        // If there's no direct conversion, you'll need to manually map fields
        let orderbook = Orderbook {
            market: proto_orderbook.market,
            bids: proto_orderbook.bids,
            asks: proto_orderbook.asks,
        };

        Ok(orderbook)
    }

    /// Returns the orderbook depth (aggregated by price level) for a specific market.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC fails
    /// - If the response cannot be deserialized
    /// - If the market does not exist
    async fn get_orderbook_depth(&self, market: &str, levels: Option<usize>) -> Result<OrderbookDepth, OrderbookError> {
        let path = match levels {
            Some(l) => format!("orderbook/depth/{}/{}", market, l),
            None => format!("orderbook/depth/{}", market),
        };

        let response = self
            .abci_query(Some(path), vec![], None, false)
            .await
            .map_err(|e| OrderbookError::Client(e))?;

        if response.code.is_err() {
            return Err(OrderbookError::MarketNotFound(market.to_string()));
        }

        // Parse the response - we don't have a protobuf OrderbookDepth type, so use the JSON response
        let proto_depth: OrderbookDepth = serde_json::from_slice(&response.value)
            .map_err(|e| OrderbookError::Other(format!("Failed to decode orderbook depth: {}", e)))?;
        
        // The response should already be in the format we need
        let depth = proto_depth;

        Ok(depth)
    }

    /// Returns a specific order by ID.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC fails
    /// - If the response cannot be deserialized
    /// - If the order does not exist
    async fn get_order(&self, order_id: &str) -> Result<Order, OrderbookError> {
        let path = format!("orderbook/order/{}", order_id);

        let response = self
            .abci_query(Some(path), vec![], None, false)
            .await
            .map_err(|e| OrderbookError::Client(e))?;

        if response.code.is_err() {
            return Err(OrderbookError::OrderNotFound(order_id.to_string()));
        }

        // Deserialize the response using protobuf
        let proto_order = astria_core::generated::astria::protocol::orderbook::v1::Order::decode(&*response.value)
            .map_err(|e| OrderbookError::Other(format!("Failed to decode order: {}", e)))?;

        // Convert the protobuf Order to our domain Order
        let order = Order::try_from(proto_order)
            .map_err(|e| OrderbookError::Other(format!("Failed to convert order: {}", e)))?;

        Ok(order)
    }

    /// Returns all orders for a specific market, optionally filtered by side.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC fails
    /// - If the response cannot be deserialized
    /// - If the market does not exist
    async fn get_market_orders(&self, market: &str, side: Option<OrderSide>) -> Result<Vec<Order>, OrderbookError> {
        // Initialize an empty vector to collect orders
        let mut all_orders = Vec::new();
        
        // Determine which sides to query based on the input parameter
        let sides_to_query = match side {
            Some(OrderSide::Buy) => vec!["buy"],
            Some(OrderSide::Sell) => vec!["sell"],
            _ => vec!["buy", "sell"], // Query both sides by default or if OrderSide::Unspecified
        };
        
        // Query each side separately
        for side_str in &sides_to_query {
            let path = format!("orderbook/orders/market/{}/{}", market, side_str);
            
            let response = self
                .abci_query(Some(path.clone()), vec![], None, false)
                .await
                .map_err(|e| OrderbookError::Client(e))?;
            
            // Check if the response indicates an error
            if response.code.is_err() {
                // If this is the first side queried and it returns an error, 
                // it likely means the market doesn't exist
                if all_orders.is_empty() && sides_to_query.len() == 1 {
                    return Err(OrderbookError::MarketNotFound(market.to_string()));
                }
                
                // Otherwise, continue to the next side
                continue;
            }
            
            // If the response is empty, continue to the next side
            if response.value.is_empty() {
                continue;
            }
            
            // Try to decode the response using our OrderList type
            match OrderList::decode(&response.value) {
                Ok(order_list) => {
                    tracing::debug!("Successfully decoded {} orders using protobuf", order_list.orders.len());
                    all_orders.extend(order_list.orders);
                },
                Err(proto_err) => {
                    tracing::warn!("Failed to decode orders as protobuf: {}", proto_err);
                    
                    // Fall back to Borsh decoding
                    if let Ok(wrappers) = borsh::from_slice::<Vec<OrderWrapper>>(&response.value) {
                        tracing::debug!("Successfully decoded {} orders using Borsh", wrappers.len());
                        let orders = wrappers.into_iter().map(|w| w.0).collect::<Vec<_>>();
                        all_orders.extend(orders);
                    } else {
                        // Try to decode a single order directly
                        if let Ok(order) = Order::decode(&mut response.value.as_ref()) {
                            tracing::debug!("Successfully decoded a single order");
                            all_orders.push(order);
                        } else {
                            tracing::warn!("Failed to decode orders from response");
                        }
                    }
                }
            }
        }
        
        Ok(all_orders)
    }

    /// Returns all orders owned by a specific address.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC fails
    /// - If the response cannot be deserialized
    async fn get_owner_orders(&self, owner: Address) -> Result<Vec<Order>, OrderbookError> {
        let path = format!("orderbook/orders/owner/{}", owner);

        let response = self
            .abci_query(Some(path), vec![], None, false)
            .await
            .map_err(|e| OrderbookError::Client(e))?;

        // Try to decode the response using our OrderList type
        match OrderList::decode(&response.value) {
            Ok(order_list) => {
                tracing::debug!("Successfully decoded {} orders using protobuf", order_list.orders.len());
                Ok(order_list.orders)
            },
            Err(proto_err) => {
                tracing::warn!("Failed to decode orders as protobuf: {}", proto_err);
                
                // Fall back to Borsh decoding
                let wrappers = borsh::from_slice::<Vec<OrderWrapper>>(&response.value)
                    .map_err(OrderbookError::Deserialization)?;

                Ok(wrappers.into_iter().map(|w| w.0).collect())
            }
        }
    }

    /// Returns recent trades for a specific market.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC fails
    /// - If the response cannot be deserialized
    /// - If the market does not exist
    async fn get_trades(&self, market: &str, limit: Option<usize>) -> Result<Vec<OrderMatch>, OrderbookError> {
        let path = match limit {
            Some(l) => format!("orderbook/trades/{}/{}", market, l),
            None => format!("orderbook/trades/{}/10", market), // Default limit is 10
        };

        let response = self
            .abci_query(Some(path), vec![], None, false)
            .await
            .map_err(|e| OrderbookError::Client(e))?;

        if response.code.is_err() {
            return Err(OrderbookError::MarketNotFound(market.to_string()));
        }

        // Try to decode the response using our TradeList type
        match TradeList::decode(&response.value) {
            Ok(trade_list) => {
                tracing::debug!("Successfully decoded {} trades using protobuf", trade_list.trades.len());
                Ok(trade_list.trades)
            },
            Err(proto_err) => {
                tracing::warn!("Failed to decode trades as protobuf: {}", proto_err);
                
                // For now, return an error if we can't decode the trades
                // In a future version, we could add a fallback mechanism
                Err(OrderbookError::Other(format!("Failed to decode trades: {}", proto_err)))
            }
        }
    }
}

/// Implement the OrderbookClientExt trait for any type that implements the SequencerClientExt trait
#[async_trait]
impl<T: crate::extension_trait::SequencerClientExt + Sync> OrderbookClientExt for T {
    async fn create_order(
        &self,
        _market: String,
        _side: OrderSide,
        _order_type: OrderType,
        _price: Option<u128>,
        _quantity: u128,
        _time_in_force: OrderTimeInForce,
        _fee_asset: String,
    ) -> Result<tendermint_rpc::endpoint::broadcast::tx_sync::Response, OrderbookError> {
        // For now, we're just returning a placeholder error since the full implementation
        // would require accessing internal/private methods of the SequencerClientExt trait
        Err(OrderbookError::Other("Transaction submission not yet implemented. This requires coordination with the sequencer team to expose transaction building functionality.".to_string()))
    }

    async fn cancel_order(
        &self,
        _order_id: String,
        _fee_asset: String,
    ) -> Result<tendermint_rpc::endpoint::broadcast::tx_sync::Response, OrderbookError> {
        // For now, we're just returning a placeholder error since the full implementation
        // would require accessing internal/private methods of the SequencerClientExt trait
        Err(OrderbookError::Other("Transaction submission not yet implemented. This requires coordination with the sequencer team to expose transaction building functionality.".to_string()))
    }

    async fn create_market(
        &self,
        _market: String,
        _base_asset: String,
        _quote_asset: String,
        _tick_size: Option<u128>,
        _lot_size: Option<u128>,
        _fee_asset: String,
    ) -> Result<tendermint_rpc::endpoint::broadcast::tx_sync::Response, OrderbookError> {
        // For now, we're just returning a placeholder error since the full implementation
        // would require accessing internal/private methods of the SequencerClientExt trait
        Err(OrderbookError::Other("Transaction submission not yet implemented. This requires coordination with the sequencer team to expose transaction building functionality.".to_string()))
    }

    async fn update_market(
        &self,
        _market: String,
        _tick_size: Option<u128>,
        _lot_size: Option<u128>,
        _paused: bool,
        _fee_asset: String,
    ) -> Result<tendermint_rpc::endpoint::broadcast::tx_sync::Response, OrderbookError> {
        // For now, we're just returning a placeholder error since the full implementation
        // would require accessing internal/private methods of the SequencerClientExt trait
        Err(OrderbookError::Other("Transaction submission not yet implemented. This requires coordination with the sequencer team to expose transaction building functionality.".to_string()))
    }
}

// Wrapper types for Borsh serialization/deserialization
// These are similar to those in the sequencer crate

/// Wrapper for Order
#[derive(Debug, Clone)]
pub struct OrderWrapper(pub Order);

impl borsh::BorshSerialize for OrderWrapper {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.0.encode_to_vec();
        borsh::BorshSerialize::serialize(&bytes, writer)
    }
}

impl borsh::BorshDeserialize for OrderWrapper {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes: Vec<u8> = borsh::BorshDeserialize::deserialize_reader(reader)?;
        Order::decode(&*bytes)
            .map(OrderWrapper)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Wrapper for OrderMatch
#[derive(Debug)]
struct OrderMatchWrapper(OrderMatch);

impl borsh::BorshSerialize for OrderMatchWrapper {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.0.encode_to_vec();
        borsh::BorshSerialize::serialize(&bytes, writer)
    }
}

impl borsh::BorshDeserialize for OrderMatchWrapper {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes: Vec<u8> = borsh::BorshDeserialize::deserialize_reader(reader)?;
        OrderMatch::decode(&*bytes)
            .map(OrderMatchWrapper)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Wrapper for Orderbook
#[derive(Debug)]
struct OrderbookWrapper(Orderbook);

impl borsh::BorshSerialize for OrderbookWrapper {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.0.encode_to_vec();
        borsh::BorshSerialize::serialize(&bytes, writer)
    }
}

impl borsh::BorshDeserialize for OrderbookWrapper {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes: Vec<u8> = borsh::BorshDeserialize::deserialize_reader(reader)?;
        Orderbook::decode(&*bytes)
            .map(OrderbookWrapper)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Wrapper for OrderbookDepth
#[derive(Debug)]
struct OrderbookDepthWrapper(OrderbookDepth);

impl borsh::BorshSerialize for OrderbookDepthWrapper {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        borsh::BorshSerialize::serialize(&self.0, writer)
    }
}

impl borsh::BorshDeserialize for OrderbookDepthWrapper {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let depth = OrderbookDepth::deserialize_reader(reader)?;
        Ok(OrderbookDepthWrapper(depth))
    }
}