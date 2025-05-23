use clap::Subcommand;
use color_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use std::str::FromStr;

use astria_core::protocol::orderbook::v1::{
    OrderSide, OrderType, OrderTimeInForce,
};
use astria_core::generated::primitive::v1::Uint128;
use astria_sequencer_client::{
    HttpClient, 
    OrderbookClientExt, 
    OrderbookError,
    tendermint_rpc,
    Order,
};
use chrono::DateTime;
use astria_core::protocol::transaction::v1::action::Action;

use crate::utils::submit_transaction;

/// Commands for interacting with the orderbook
#[derive(Debug, Subcommand)]
pub enum SubCommand {
    /// Create a new trading market
    CreateMarket(CreateMarketCommand),
    /// Create a new order
    CreateOrder(CreateOrderCommand),
    /// Cancel an existing order
    CancelOrder(CancelOrderCommand),
    /// Get all markets
    GetMarkets(GetMarketsCommand),
    /// Get market parameters
    GetMarketParams(GetMarketParamsCommand),
    /// Get orders for a specific market
    GetOrders(GetOrdersCommand),
    /// Get order details
    GetOrder(GetOrderCommand),
    /// Get orderbook for a market
    GetOrderbook(GetOrderbookCommand),
    /// Get orderbook depth for a market
    GetOrderbookDepth(GetOrderbookDepthCommand),
    /// Get recent trades for a market
    GetTrades(GetTradesCommand),
}

#[derive(Debug, clap::Args)]
pub struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::CreateMarket(create_market) => create_market.run().await,
            SubCommand::CreateOrder(create_order) => create_order.run().await,
            SubCommand::CancelOrder(cancel_order) => cancel_order.run().await,
            SubCommand::GetMarkets(get_markets) => get_markets.run().await,
            SubCommand::GetMarketParams(get_market_params) => get_market_params.run().await,
            SubCommand::GetOrders(get_orders) => get_orders.run().await,
            SubCommand::GetOrder(get_order) => get_order.run().await,
            SubCommand::GetOrderbook(get_orderbook) => get_orderbook.run().await,
            SubCommand::GetOrderbookDepth(get_orderbook_depth) => get_orderbook_depth.run().await,
            SubCommand::GetTrades(get_trades) => get_trades.run().await,
        }
    }
}

/// Parse a side string to OrderSide
fn parse_side(side_str: &str) -> OrderSide {
    match side_str.to_lowercase().as_str() {
        "buy" => OrderSide::Buy,
        "sell" => OrderSide::Sell,
        _ => panic!("Invalid side: {}", side_str),
    }
}

/// Format a side value as a string
fn format_side(side: i32) -> String {
    let side_enum = match side {
        1 => OrderSide::Buy,
        2 => OrderSide::Sell,
        _ => OrderSide::Unspecified,
    };
    match side_enum {
        OrderSide::Buy => "BUY".to_string(),
        OrderSide::Sell => "SELL".to_string(),
        _ => "UNKNOWN".to_string(),
    }
}

/// Format an order type as a string
fn format_order_type(order_type: i32) -> String {
    let type_enum = match order_type {
        1 => OrderType::Limit,
        2 => OrderType::Market,
        _ => OrderType::Unspecified,
    };
    match type_enum {
        OrderType::Limit => "LIMIT".to_string(),
        OrderType::Market => "MARKET".to_string(),
        _ => "UNKNOWN".to_string(),
    }
}

/// Format a time in force value as a string
fn format_time_in_force(tif: i32) -> String {
    let tif_enum = match tif {
        1 => OrderTimeInForce::Gtc,
        2 => OrderTimeInForce::Ioc,
        3 => OrderTimeInForce::Fok,
        _ => OrderTimeInForce::Unspecified,
    };
    match tif_enum {
        OrderTimeInForce::Gtc => "Good Till Cancelled (GTC)".to_string(),
        OrderTimeInForce::Ioc => "Immediate Or Cancel (IOC)".to_string(),
        OrderTimeInForce::Fok => "Fill Or Kill (FOK)".to_string(),
        _ => "UNKNOWN".to_string(),
    }
}

/// Format a u128 as a string
fn format_u128(value: u128) -> String {
    value.to_string()
}

/// Format a quantity (Uint128) for display
fn format_quantity(quantity: &Option<Uint128>) -> String {
    if let Some(qty) = quantity {
        let qty_value = ((qty.hi as u128) << 64) + (qty.lo as u128);
        qty_value.to_string()
    } else {
        "Not specified".to_string()
    }
}

/// Format a price (Uint128) for display
fn format_price(price: &Option<Uint128>) -> String {
    if let Some(price) = price {
        let price_value = ((price.hi as u128) << 64) + (price.lo as u128);
        price_value.to_string()
    } else {
        "Not specified".to_string()
    }
}

/// Format a timestamp as a string
fn format_timestamp(timestamp: u64) -> String {
    if timestamp == 0 {
        return "N/A".to_string();
    }
    
    match DateTime::<chrono::Utc>::from_timestamp(timestamp as i64, 0) {
        Some(dt) => dt.to_rfc3339(),
        None => timestamp.to_string(),
    }
}

/// Format an order status as a string
fn format_order_status(status: i32) -> String {
    match status {
        0 => "UNKNOWN".to_string(),
        1 => "OPEN".to_string(),
        2 => "FILLED".to_string(),
        3 => "CANCELLED".to_string(),
        4 => "EXPIRED".to_string(),
        _ => format!("UNKNOWN ({})", status),
    }
}

/// Get all markets
#[derive(Debug, clap::Args)]
struct GetMarketsCommand {
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL", default_value = "http://localhost:26657")]
    sequencer_url: String,
}

impl GetMarketsCommand {
    async fn run(self) -> eyre::Result<()> {
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let sequencer_client = HttpClient::new(url)
            .wrap_err("failed constructing http sequencer client")?;
        
        println!("Querying available markets from sequencer at {}", self.sequencer_url);
        
        match sequencer_client.get_markets().await {
            Ok(markets) => {
                if markets.is_empty() {
                    println!("No markets found");
                } else {
                    println!("Available markets:");
                    for (i, market) in markets.iter().enumerate() {
                        println!("{}. {}", i + 1, market);
                    }
                }
            },
            Err(e) => {
                return Err(eyre!("Failed to get markets: {}", e));
            }
        }
        
        Ok(())
    }
}

/// Get market parameters
#[derive(Debug, clap::Args)]
struct GetMarketParamsCommand {
    /// Market identifier, e.g., BTC/USD
    market: String,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL", default_value = "http://localhost:26657")]
    sequencer_url: String,
}

impl GetMarketParamsCommand {
    async fn run(self) -> eyre::Result<()> {
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let sequencer_client = HttpClient::new(url)
            .wrap_err("failed constructing http sequencer client")?;
        
        println!("Querying parameters for market {} from sequencer at {}", self.market, self.sequencer_url);
        
        match sequencer_client.get_market_params(&self.market).await {
            Ok(params) => {
                println!("Market parameters for {}:", self.market);
                println!("  Base Asset: {}", params.base_asset);
                println!("  Quote Asset: {}", params.quote_asset);
                println!("  Tick Size: {}", params.tick_size.map_or_else(|| "None".to_string(), format_u128));
                println!("  Lot Size: {}", params.lot_size.map_or_else(|| "None".to_string(), format_u128));
                println!("  Paused: {}", params.paused);
            },
            Err(e) => {
                match e {
                    OrderbookError::MarketNotFound(_) => {
                        println!("Market {} not found", self.market);
                    },
                    _ => {
                        return Err(eyre!("Failed to get market parameters: {}", e));
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Display format for orders
#[derive(Debug, Clone, Copy)]
enum DisplayFormat {
    Simple,
    Detailed,
    Json,
}

impl FromStr for DisplayFormat {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "simple" => Ok(DisplayFormat::Simple),
            "detailed" => Ok(DisplayFormat::Detailed),
            "json" => Ok(DisplayFormat::Json),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

/// Get orders for a specific market
#[derive(Debug, clap::Args)]
struct GetOrdersCommand {
    /// Market identifier, e.g., BTC/USD
    market: String,
    /// Order side (buy or sell) - optional
    #[arg(long)]
    side: Option<String>,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL", default_value = "http://localhost:26657")]
    sequencer_url: String,
    /// Output format (simple, json, detailed)
    #[arg(long, default_value = "detailed")]
    format: String,
}

impl GetOrdersCommand {
    async fn run(self) -> eyre::Result<()> {
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let sequencer_client = HttpClient::new(url)
            .wrap_err("failed constructing http sequencer client")?;
        
        println!("Querying orders for market {} from sequencer at {}", self.market, self.sequencer_url);
        
        // Parse the side parameter
        let side = match &self.side {
            Some(s) => {
                match s.to_lowercase().as_str() {
                    "buy" => Some(OrderSide::Buy),
                    "sell" => Some(OrderSide::Sell),
                    _ => {
                        return Err(eyre!("Invalid side: {}. Must be 'buy' or 'sell'", s));
                    }
                }
            },
            None => None,
        };
        
        println!("Querying for market: {}, side: {:?}", self.market, side);
        
        // Parse the format parameter
        let format = DisplayFormat::from_str(&self.format)
            .map_err(|e| eyre!(e))?;
        
        // Use the client's get_market_orders method to get orders
        println!("Querying for market: {}, side: {:?}", self.market, side);
        
        // Try to get the orders
        let orders = match sequencer_client.get_market_orders(&self.market, side).await {
            Ok(orders) => orders,
            Err(e) => {
                match e {
                    OrderbookError::MarketNotFound(_) => {
                        println!("Market {} not found", self.market);
                        return Ok(());
                    },
                    _ => {
                        return Err(eyre!("Failed to get orders: {}", e));
                    }
                }
            }
        };
        
        // If we didn't find any orders
        if orders.is_empty() {
            println!("No orders found for market {}", self.market);
            return Ok(());
        }
        
        // Display the orders
        match format {
            DisplayFormat::Simple => {
                println!("\nOrders found (ID only):");
                for (i, order) in orders.iter().enumerate() {
                    println!("{}. {}", i + 1, order.id);
                }
            },
            DisplayFormat::Json => {
                println!("\nOrders in JSON format:");
                let json = serde_json::to_string_pretty(&orders)
                    .wrap_err("failed to serialize orders to JSON")?;
                println!("{}", json);
            },
            DisplayFormat::Detailed => {
                if orders.is_empty() {
                    println!("No orders found for market {}", self.market);
                } else {
                    println!("\nOrders found:");
                    for (i, order) in orders.iter().enumerate() {
                        println!("Order {}:", i + 1);
                        println!("  ID: {}", order.id);
                        println!("  Market: {}", order.market);
                        println!("  Side: {}", format_side(order.side));
                        println!("  Type: {}", format_order_type(order.r#type));
                        println!("  Price: {}", format_price(&order.price));
                        println!("  Quantity: {}", format_quantity(&order.quantity));
                        println!("  Remaining Quantity: {}", format_quantity(&order.remaining_quantity));
                        
                        if let Some(owner) = &order.owner {
                            println!("  Owner: {}", owner.bech32m);
                        } else {
                            println!("  Owner: Unknown");
                        }
                        
                        println!("  Created At: {}", format_timestamp(order.created_at));
                        println!("  Time In Force: {}", format_time_in_force(order.time_in_force));
                        // Orders currently don't have a status field, so use "OPEN" as default
                        println!("  Status: {}", format_order_status(1));
                        println!("");
                    }
                }
            }
        }
        
        // Count BUY and SELL orders separately for debugging
        let buy_count = orders.iter().filter(|o| o.side == 1).count();
        let sell_count = orders.iter().filter(|o| o.side == 2).count();
        println!("Found {} total orders in market {} ({} BUY, {} SELL)", 
            orders.len(), self.market, buy_count, sell_count);
        
        Ok(())
    }
}

/// Get order details
#[derive(Debug, clap::Args)]
struct GetOrderCommand {
    /// Order ID(s) to retrieve (can provide multiple)
    #[arg(required = true, num_args = 1..)]
    order_ids: Vec<String>,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL", default_value = "http://localhost:26657")]
    sequencer_url: String,
    /// Output format (simple, json, detailed)
    #[arg(long, default_value = "detailed")]
    format: String,
}

impl GetOrderCommand {
    async fn run(self) -> eyre::Result<()> {
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let sequencer_client = HttpClient::new(url.clone())
            .wrap_err("failed constructing http sequencer client")?;
        
        if self.order_ids.is_empty() {
            println!("No order IDs provided");
            return Ok(());
        }
        
        println!("Querying {} order(s) from sequencer at {}", self.order_ids.len(), self.sequencer_url);
        
        // Parse the format parameter
        let format = DisplayFormat::from_str(&self.format)
            .map_err(|e| eyre!(e))?;
        
        // Request each order separately and collect the results
        let mut successful_orders = Vec::new();
        let mut failed_order_ids = Vec::new();

        for order_id in &self.order_ids {
            println!("Querying order: {}", order_id);
            
            match sequencer_client.get_order(order_id).await {
                Ok(order) => {
                    successful_orders.push(order);
                },
                Err(e) => {
                    match e {
                        OrderbookError::OrderNotFound(_) => {
                            println!("Order {} not found", order_id);
                        },
                        _ => {
                            println!("Error getting order {}: {}", order_id, e);
                        }
                    }
                    failed_order_ids.push(order_id.clone());
                }
            }
        }
        
        // Display the results based on the selected format
        if !successful_orders.is_empty() {
            match format {
                DisplayFormat::Simple => {
                    println!("\nOrders found (ID only):");
                    for (i, order) in successful_orders.iter().enumerate() {
                        println!("{}. {}", i + 1, order.id);
                    }
                },
                DisplayFormat::Json => {
                    println!("\nOrders in JSON format:");
                    let json = serde_json::to_string_pretty(&successful_orders)
                        .wrap_err("failed to serialize orders to JSON")?;
                    println!("{}", json);
                },
                DisplayFormat::Detailed => {
                    println!("\nOrders found:");
                    for (i, order) in successful_orders.iter().enumerate() {
                        println!("Order {}:", i + 1);
                        println!("  ID: {}", order.id);
                        println!("  Market: {}", order.market);
                        println!("  Side: {}", format_side(order.side));
                        println!("  Type: {}", format_order_type(order.r#type));
                        println!("  Price: {}", format_price(&order.price));
                        println!("  Quantity: {}", format_quantity(&order.quantity));
                        println!("  Remaining Quantity: {}", format_quantity(&order.remaining_quantity));
                        
                        if let Some(owner) = &order.owner {
                            println!("  Owner: {}", owner.bech32m);
                        } else {
                            println!("  Owner: Unknown");
                        }
                        
                        println!("  Created At: {}", format_timestamp(order.created_at));
                        println!("  Time In Force: {}", format_time_in_force(order.time_in_force));
                        // Orders currently don't have a status field, so use "OPEN" as default
                        println!("  Status: {}", format_order_status(1));
                        
                        println!("");
                    }
                }
            }
            
            println!("Successfully retrieved {} order(s)", successful_orders.len());
        } else {
            println!("No orders were successfully retrieved");
        }
        
        if !failed_order_ids.is_empty() {
            println!("Failed to retrieve {} order(s): {:?}", failed_order_ids.len(), failed_order_ids);
        }
        
        Ok(())
    }
}

/// Get orderbook for a market
#[derive(Debug, clap::Args)]
struct GetOrderbookCommand {
    /// Market identifier, e.g., BTC/USD
    market: String,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL", default_value = "http://localhost:26657")]
    sequencer_url: String,
    /// Output format (simple, json, detailed)
    #[arg(long, default_value = "detailed")]
    format: String,
}

impl GetOrderbookCommand {
    async fn run(self) -> eyre::Result<()> {
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let sequencer_client = HttpClient::new(url)
            .wrap_err("failed constructing http sequencer client")?;
        
        println!("Querying orderbook for market {} from sequencer at {}", self.market, self.sequencer_url);
        
        // Parse the format parameter
        let format = DisplayFormat::from_str(&self.format)
            .map_err(|e| eyre!(e))?;
        
        match sequencer_client.get_orderbook(&self.market).await {
            Ok(orderbook) => {
                match format {
                    DisplayFormat::Simple => {
                        println!("\nOrderbook summary for market {}:", self.market);
                        println!("Bids (Buy orders): {}", orderbook.bids.len());
                        println!("Asks (Sell orders): {}", orderbook.asks.len());
                    },
                    DisplayFormat::Json => {
                        println!("\nOrderbook in JSON format:");
                        let json = serde_json::to_string_pretty(&orderbook)
                            .wrap_err("failed to serialize orderbook to JSON")?;
                        println!("{}", json);
                    },
                    DisplayFormat::Detailed => {
                        println!("\nOrderbook for market {}:", self.market);
                        
                        if orderbook.bids.is_empty() && orderbook.asks.is_empty() {
                            println!("The orderbook is empty");
                        } else {
                            if !orderbook.bids.is_empty() {
                                println!("\nBids (Buy orders):");
                                for (i, entry) in orderbook.bids.iter().enumerate() {
                                    println!("Order Level {}:", i + 1);
                                    
                                    if let Some(price) = &entry.price {
                                        let price_value = ((price.hi as u128) << 64) + (price.lo as u128);
                                        println!("  Price: {}", price_value);
                                    } else {
                                        println!("  Price: Not specified");
                                    }
                                    
                                    if let Some(quantity) = &entry.quantity {
                                        let quantity_value = ((quantity.hi as u128) << 64) + (quantity.lo as u128);
                                        println!("  Quantity: {}", quantity_value);
                                    } else {
                                        println!("  Quantity: Not specified");
                                    }
                                    
                                    println!("  Order Count: {}", entry.order_count);
                                    println!("");
                                }
                            }
                            
                            if !orderbook.asks.is_empty() {
                                println!("\nAsks (Sell orders):");
                                for (i, entry) in orderbook.asks.iter().enumerate() {
                                    println!("Order Level {}:", i + 1);
                                    
                                    if let Some(price) = &entry.price {
                                        let price_value = ((price.hi as u128) << 64) + (price.lo as u128);
                                        println!("  Price: {}", price_value);
                                    } else {
                                        println!("  Price: Not specified");
                                    }
                                    
                                    if let Some(quantity) = &entry.quantity {
                                        let quantity_value = ((quantity.hi as u128) << 64) + (quantity.lo as u128);
                                        println!("  Quantity: {}", quantity_value);
                                    } else {
                                        println!("  Quantity: Not specified");
                                    }
                                    
                                    println!("  Order Count: {}", entry.order_count);
                                    println!("");
                                }
                            }
                        }
                    }
                }
            },
            Err(e) => {
                match e {
                    OrderbookError::MarketNotFound(_) => {
                        println!("Market {} not found", self.market);
                    },
                    _ => {
                        return Err(eyre!("Failed to get orderbook: {}", e));
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Get orderbook depth for a market
#[derive(Debug, clap::Args)]
struct GetOrderbookDepthCommand {
    /// Market identifier, e.g., BTC/USD
    market: String,
    /// Number of price levels to return
    #[arg(long)]
    levels: Option<usize>,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL", default_value = "http://localhost:26657")]
    sequencer_url: String,
    /// Output format (simple, json, detailed)
    #[arg(long, default_value = "detailed")]
    format: String,
}

impl GetOrderbookDepthCommand {
    async fn run(self) -> eyre::Result<()> {
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let sequencer_client = HttpClient::new(url)
            .wrap_err("failed constructing http sequencer client")?;
        
        println!("Querying orderbook depth for market {} from sequencer at {}", self.market, self.sequencer_url);
        if let Some(levels) = self.levels {
            println!("Getting top {} price levels", levels);
        }
        
        // Parse the format parameter
        let format = DisplayFormat::from_str(&self.format)
            .map_err(|e| eyre!(e))?;
        
        match sequencer_client.get_orderbook_depth(&self.market, self.levels).await {
            Ok(depth) => {
                match format {
                    DisplayFormat::Simple => {
                        println!("\nOrderbook depth summary for market {}:", self.market);
                        println!("Bid levels (Buy orders): {}", depth.bids.len());
                        println!("Ask levels (Sell orders): {}", depth.asks.len());
                    },
                    DisplayFormat::Json => {
                        println!("\nOrderbook depth in JSON format:");
                        let json = serde_json::to_string_pretty(&depth)
                            .wrap_err("failed to serialize orderbook depth to JSON")?;
                        println!("{}", json);
                    },
                    DisplayFormat::Detailed => {
                        println!("\nOrderbook depth for market {}:", self.market);
                        
                        if depth.bids.is_empty() && depth.asks.is_empty() {
                            println!("The orderbook is empty");
                        } else {
                            if !depth.bids.is_empty() {
                                println!("\nBids (Buy orders):");
                                println!("  Price Level  |  Quantity  |  Orders");
                                println!("  ------------------------------------");
                                for level in &depth.bids {
                                    let price = level.price.map(format_u128).unwrap_or_else(|| "N/A".to_string());
                                    let quantity = level.quantity.map(format_u128).unwrap_or_else(|| "N/A".to_string());
                                    println!("  {:12}  |  {:8}  |  {}", price, quantity, level.order_count);
                                }
                            }
                            
                            if !depth.asks.is_empty() {
                                println!("\nAsks (Sell orders):");
                                println!("  Price Level  |  Quantity  |  Orders");
                                println!("  ------------------------------------");
                                for level in &depth.asks {
                                    let price = level.price.map(format_u128).unwrap_or_else(|| "N/A".to_string());
                                    let quantity = level.quantity.map(format_u128).unwrap_or_else(|| "N/A".to_string());
                                    println!("  {:12}  |  {:8}  |  {}", price, quantity, level.order_count);
                                }
                            }
                        }
                    }
                }
            },
            Err(e) => {
                match e {
                    OrderbookError::MarketNotFound(_) => {
                        println!("Market {} not found", self.market);
                    },
                    _ => {
                        return Err(eyre!("Failed to get orderbook depth: {}", e));
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Get recent trades for a market
#[derive(Debug, clap::Args)]
struct GetTradesCommand {
    /// Market identifier, e.g., BTC/USD
    market: String,
    /// Maximum number of trades to return
    #[arg(long)]
    limit: Option<usize>,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL", default_value = "http://localhost:26657")]
    sequencer_url: String,
    /// Output format (simple, json, detailed)
    #[arg(long, default_value = "detailed")]
    format: String,
}

impl GetTradesCommand {
    async fn run(self) -> eyre::Result<()> {
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let sequencer_client = HttpClient::new(url)
            .wrap_err("failed constructing http sequencer client")?;
        
        println!("Querying recent trades for market {} from sequencer at {}", self.market, self.sequencer_url);
        if let Some(limit) = self.limit {
            println!("Getting up to {} trades", limit);
        }
        
        // Parse the format parameter
        let format = DisplayFormat::from_str(&self.format)
            .map_err(|e| eyre!(e))?;
        
        match sequencer_client.get_trades(&self.market, self.limit).await {
            Ok(trades) => {
                match format {
                    DisplayFormat::Simple => {
                        println!("\nRecent trades for market {}:", self.market);
                        println!("Found {} trades", trades.len());
                    },
                    DisplayFormat::Json => {
                        println!("\nTrades in JSON format:");
                        let json = serde_json::to_string_pretty(&trades)
                            .wrap_err("failed to serialize trades to JSON")?;
                        println!("{}", json);
                    },
                    DisplayFormat::Detailed => {
                        if trades.is_empty() {
                            println!("No trades found for market {}", self.market);
                        } else {
                            println!("\nRecent trades for market {}:", self.market);
                            for (i, trade) in trades.iter().enumerate() {
                                println!("Trade {}:", i + 1);
                                println!("  Market: {}", trade.market);
                                
                                if let Some(price) = &trade.price {
                                    let price_value = ((price.hi as u128) << 64) + (price.lo as u128);
                                    println!("  Price: {}", price_value);
                                } else {
                                    println!("  Price: Not specified");
                                }
                                
                                if let Some(quantity) = &trade.quantity {
                                    let quantity_value = ((quantity.hi as u128) << 64) + (quantity.lo as u128);
                                    println!("  Quantity: {}", quantity_value);
                                } else {
                                    println!("  Quantity: Not specified");
                                }
                                
                                println!("  Timestamp: {}", format_timestamp(trade.timestamp));
                                println!("  Maker Order ID: {}", trade.maker_order_id);
                                println!("  Taker Order ID: {}", trade.taker_order_id);
                                println!("  Taker Side: {}", match trade.taker_side {
                                    1 => "BUY",
                                    2 => "SELL",
                                    _ => "UNKNOWN"
                                });
                                
                                println!("");
                            }
                            
                            println!("Found {} trades for market {}", trades.len(), self.market);
                        }
                    }
                }
            },
            Err(e) => {
                match e {
                    OrderbookError::MarketNotFound(_) => {
                        println!("Market {} not found", self.market);
                    },
                    _ => {
                        return Err(eyre!("Failed to get trades: {}", e));
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Create a new trading market
#[derive(Debug, clap::Args)]
struct CreateMarketCommand {
    /// Market identifier, e.g., BTC/USD
    market: String,
    /// Base asset, e.g., BTC
    base_asset: String,
    /// Quote asset, e.g., USD
    quote_asset: String,
    /// Minimum price increment, e.g., 0.01
    tick_size: String,
    /// Minimum quantity increment, e.g., 0.001
    lot_size: String,
    /// Asset to pay fees in
    #[arg(long, default_value = "nria")]
    fee_asset: String,
    /// RPC endpoint
    #[arg(short, long, default_value = "http://localhost:26657")]
    sequencer_url: String,
    /// Chain ID
    #[arg(long, default_value = "astria")]
    chain_id: String,
    /// Prefix for addresses
    #[arg(long, default_value = "astria")]
    prefix: String,
    /// Private key (hex-encoded)
    #[arg(short, long, env = "ASTRIA_CLI_SEQUENCER_PRIVATE_KEY")]
    private_key: String,
}

impl CreateMarketCommand {
    async fn run(self) -> eyre::Result<()> {
        // Constructing the action JSON manually for now
        // This avoids having to deal with complex type conversions
        let action_json = format!(
            r#"{{
                "createMarket": {{
                    "market": "{}",
                    "base_asset": "{}",
                    "quote_asset": "{}",
                    "tick_size": {{
                        "lo": "{}",
                        "hi": "0"
                    }},
                    "lot_size": {{
                        "lo": "{}",
                        "hi": "0"
                    }},
                    "fee_asset": "{}"
                }}
            }}"#,
            self.market, self.base_asset, self.quote_asset, 
            self.tick_size, self.lot_size, self.fee_asset
        );

        let action: Action = serde_json::from_str(&action_json)
            .wrap_err("failed to construct action from JSON")?;

        // First get a client to check transaction status
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let _sequencer_client = HttpClient::new(url)
            .wrap_err("failed constructing http sequencer client")?;
            
        println!("Submitting transaction to create market...");
        
        let hash = match submit_transaction(
            &self.sequencer_url,
            self.chain_id,
            &self.prefix,
            &self.private_key,
            action,
        )
        .await {
            Ok(response) => {
                println!("Transaction submitted successfully!");
                println!("Transaction hash: {}", response.hash);
                println!("Block height: {}", response.height);
                response.hash
            },
            Err(e) => {
                println!("Transaction submission completed with error: {}", e);
                println!("The transaction may still have been accepted.");
                println!("Check the sequencer logs for more information.");
                return Err(e);
            }
        };
        
        // Try to query the transaction status
        println!("You can query this transaction with:");
        println!("curl -s '{}/tx?hash=0x{}' | jq", self.sequencer_url, hex::encode(hash.as_bytes()));

        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct CreateOrderCommand {
    /// Market identifier, e.g., BTC/USD
    market: String,
    /// Order side (BUY or SELL)
    side: String,
    /// Order type (LIMIT or MARKET)
    order_type: String,
    /// Order price (required for limit orders)
    price: String,
    /// Order quantity
    quantity: String,
    /// Time in force (GTC, IOC, or FOK)
    time_in_force: String,
    /// Asset to pay fees in
    #[arg(long, default_value = "nria")]
    fee_asset: String,
    /// RPC endpoint
    #[arg(short, long, default_value = "http://localhost:26657")]
    sequencer_url: String,
    /// Chain ID
    #[arg(long, default_value = "astria")]
    chain_id: String,
    /// Prefix for addresses
    #[arg(long, default_value = "astria")]
    prefix: String,
    /// Private key (hex-encoded)
    #[arg(short, long, env = "ASTRIA_CLI_SEQUENCER_PRIVATE_KEY")]
    private_key: String,
}

impl CreateOrderCommand {
    async fn run(self) -> eyre::Result<()> {
        // Map side string to the expected enum value
        let side_value = match self.side.to_uppercase().as_str() {
            "BUY" => 1,
            "SELL" => 2,
            _ => return Err(eyre!("Invalid order side. Must be BUY or SELL")),
        };

        // Map order type string to the expected enum value
        let order_type_value = match self.order_type.to_uppercase().as_str() {
            "LIMIT" => 1,
            "MARKET" => 2,
            _ => return Err(eyre!("Invalid order type. Must be LIMIT or MARKET")),
        };

        // Map time in force string to the expected enum value
        let time_in_force_value = match self.time_in_force.to_uppercase().as_str() {
            "GTC" => 1,
            "IOC" => 2,
            "FOK" => 3,
            _ => return Err(eyre!("Invalid time in force. Must be GTC, IOC, or FOK")),
        };

        // Parse the user input values as-is, without scaling
        let price_value = self.price.parse::<u64>().unwrap_or(1);
        let quantity_value = self.quantity.parse::<u64>().unwrap_or(1);
        
        println!("Creating order with price {}, quantity {}", price_value, quantity_value);
        
        // Constructing the action JSON manually
        let action_json = format!(
            r#"{{
                "createOrder": {{
                    "market": "{}",
                    "side": {},
                    "type": {},
                    "price": {{
                        "lo": "{}",
                        "hi": "0"
                    }},
                    "quantity": {{
                        "lo": "{}",
                        "hi": "0"
                    }},
                    "time_in_force": {},
                    "fee_asset": "{}"
                }}
            }}"#,
            self.market, side_value, order_type_value, 
            price_value, 
            quantity_value, 
            time_in_force_value, self.fee_asset
        );

        let action: Action = serde_json::from_str(&action_json)
            .wrap_err("failed to construct action from JSON")?;

        // First get a client to check transaction status
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let _sequencer_client = HttpClient::new(url)
            .wrap_err("failed constructing http sequencer client")?;
            
        // Debug output showing the values
        println!("Submitting transaction to create order (price: {}, quantity: {})...",
            self.price,
            self.quantity
        );
        
        let hash = match submit_transaction(
            &self.sequencer_url,
            self.chain_id,
            &self.prefix,
            &self.private_key,
            action,
        )
        .await {
            Ok(response) => {
                println!("Transaction submitted successfully!");
                println!("Transaction hash: {}", response.hash);
                println!("Block height: {}", response.height);
                response.hash
            },
            Err(e) => {
                println!("Transaction submission completed with error: {}", e);
                println!("The transaction may still have been accepted.");
                println!("Check the sequencer logs for more information.");
                return Err(e);
            }
        };
        
        // Try to query the transaction status
        println!("You can query this transaction with:");
        println!("curl -s '{}/tx?hash=0x{}' | jq", self.sequencer_url, hex::encode(hash.as_bytes()));

        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct CancelOrderCommand {
    /// Order ID to cancel
    order_id: String,
    /// Asset to pay fees in
    #[arg(long, default_value = "nria")]
    fee_asset: String,
    /// RPC endpoint
    #[arg(short, long, default_value = "http://localhost:26657")]
    sequencer_url: String,
    /// Chain ID
    #[arg(long, default_value = "astria")]
    chain_id: String,
    /// Prefix for addresses
    #[arg(long, default_value = "astria")]
    prefix: String,
    /// Private key (hex-encoded)
    #[arg(short, long, env = "ASTRIA_CLI_SEQUENCER_PRIVATE_KEY")]
    private_key: String,
}

impl CancelOrderCommand {
    async fn run(self) -> eyre::Result<()> {
        // Constructing the action JSON manually
        let action_json = format!(
            r#"{{
                "cancelOrder": {{
                    "order_id": "{}",
                    "fee_asset": "{}"
                }}
            }}"#,
            self.order_id, self.fee_asset
        );

        let action: Action = serde_json::from_str(&action_json)
            .wrap_err("failed to construct action from JSON")?;

        // First get a client to check transaction status
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let _sequencer_client = HttpClient::new(url)
            .wrap_err("failed constructing http sequencer client")?;
            
        println!("Submitting transaction to cancel order...");
        
        let hash = match submit_transaction(
            &self.sequencer_url,
            self.chain_id,
            &self.prefix,
            &self.private_key,
            action,
        )
        .await {
            Ok(response) => {
                println!("Transaction submitted successfully!");
                println!("Transaction hash: {}", response.hash);
                println!("Block height: {}", response.height);
                response.hash
            },
            Err(e) => {
                println!("Transaction submission completed with error: {}", e);
                println!("The transaction may still have been accepted.");
                println!("Check the sequencer logs for more information.");
                return Err(e);
            }
        };
        
        // Try to query the transaction status
        println!("You can query this transaction with:");
        println!("curl -s '{}/tx?hash=0x{}' | jq", self.sequencer_url, hex::encode(hash.as_bytes()));

        Ok(())
    }
}