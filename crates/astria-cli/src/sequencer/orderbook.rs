use clap::Subcommand;
use color_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use std::str::FromStr;

use astria_core::protocol::transaction::v1::action::Action;
use astria_sequencer_client::{
    Client as _,
    HttpClient,
    tendermint_rpc,
};

use crate::utils::submit_transaction;

#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::CreateMarket(create_market) => create_market.run().await,
            SubCommand::CreateOrder(create_order) => create_order.run().await,
            SubCommand::CancelOrder(cancel_order) => cancel_order.run().await,
            SubCommand::GetMarkets(get_markets) => get_markets.run().await,
            SubCommand::GetOrders(get_orders) => get_orders.run().await,
        }
    }
}

/// Interact with the Sequencer orderbook
#[derive(Debug, Subcommand)]
enum SubCommand {
    /// Create a new trading market
    CreateMarket(CreateMarketCommand),
    /// Create a new order
    CreateOrder(CreateOrderCommand),
    /// Cancel an existing order
    CancelOrder(CancelOrderCommand),
    /// Get a list of available markets
    GetMarkets(GetMarketsCommand),
    /// Get orders for a specific market
    GetOrders(GetOrdersCommand),
}

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
        let sequencer_client = HttpClient::new(url)
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
            self.price, self.quantity, time_in_force_value, self.fee_asset
        );

        let action: Action = serde_json::from_str(&action_json)
            .wrap_err("failed to construct action from JSON")?;

        // First get a client to check transaction status
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let sequencer_client = HttpClient::new(url)
            .wrap_err("failed constructing http sequencer client")?;
            
        println!("Submitting transaction to create order...");
        
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
        let sequencer_client = HttpClient::new(url)
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
        
        println!("Querying markets from sequencer at {}", self.sequencer_url);
        
        // Print the current version of the sequencer to check features
        let version_response = match sequencer_client.abci_info().await {
            Ok(info) => format!("Sequencer version: {}, App version: {}", info.version, info.app_version),
            Err(e) => format!("Failed to get sequencer version: {}", e),
        };
        println!("{}", version_response);
        
        // First, try to get all component state to check if orderbook is available
        let components_response = sequencer_client
            .abci_query(Some("app/components".to_string()), vec![], Some(0u32.into()), false)
            .await
            .wrap_err("failed to query app components")?;
        
        let components_str = String::from_utf8(components_response.value.to_vec())
            .wrap_err("failed to convert components response to string")?;
        
        println!("Available components: {}", components_str);
        
        // Query the orderbook state
        println!("Querying orderbook markets...");
        let response = sequencer_client
            .abci_query(Some("orderbook/markets".to_string()), vec![], Some(0u32.into()), false)
            .await
            .wrap_err("failed to query markets")?;
        
        println!("Got response - code: {:?}, log: {}", response.code, response.log);
        println!("Response value size: {} bytes", response.value.len());
        
        // Parse the response as JSON for display
        let markets_json = String::from_utf8(response.value.to_vec())
            .wrap_err("failed to convert response to string")?;
        
        println!("Raw response: {}", markets_json);
        
        if markets_json.is_empty() || markets_json == "null" {
            println!("No markets found in response.");
            
            // Try alternative query paths
            println!("Trying alternative query paths...");
            
            for path in &[
                "orderbook", 
                "state/orderbook", 
                "app/orderbook", 
                "orderbook/state", 
                "market", 
                "markets", 
                "components", 
                "component/orderbook", 
                "abci_info"
            ] {
                println!("Trying path: {}", path);
                match sequencer_client
                    .abci_query(Some(path.to_string()), vec![], Some(0u32.into()), false)
                    .await {
                        Ok(alt_response) => {
                            if !alt_response.value.is_empty() {
                                println!("Got response from {}: {}", path, 
                                    String::from_utf8_lossy(&alt_response.value));
                            } else {
                                println!("Empty response from {}.", path);
                            }
                        },
                        Err(e) => {
                            println!("Error querying {}: {}", path, e);
                        }
                    }
            }
            return Ok(());
        }
        
        // Try to parse as JSON to pretty-print
        match serde_json::from_str::<serde_json::Value>(&markets_json) {
            Ok(json_value) => {
                println!("Markets:");
                println!("{}", serde_json::to_string_pretty(&json_value)?);
            },
            Err(e) => {
                println!("Failed to parse response as JSON: {}", e);
                println!("Raw response: {}", markets_json);
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct GetOrdersCommand {
    /// Market identifier, e.g., BTC/USD
    market: String,
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL", default_value = "http://localhost:26657")]
    sequencer_url: String,
}

impl GetOrdersCommand {
    async fn run(self) -> eyre::Result<()> {
        let url = tendermint_rpc::Url::from_str(&self.sequencer_url)
            .wrap_err("failed to parse sequencer URL")?;
        let sequencer_client = HttpClient::new(url)
            .wrap_err("failed constructing http sequencer client")?;
        
        println!("Querying orders for market {} from sequencer at {}", self.market, self.sequencer_url);
        
        let path = format!("orderbook/orders/{}", self.market);
        println!("Using query path: {}", path);
        
        let response = sequencer_client
            .abci_query(Some(path.clone()), vec![], Some(0u32.into()), false)
            .await
            .wrap_err("failed to query orderbook")?;
        
        println!("Got response - code: {:?}, log: {}", response.code, response.log);
        println!("Response value size: {} bytes", response.value.len());
        
        // Parse the response as JSON for display
        let orderbook_json = String::from_utf8(response.value.to_vec())
            .wrap_err("failed to convert response to string")?;
        
        println!("Raw response: {}", orderbook_json);
        
        if orderbook_json.is_empty() || orderbook_json == "null" {
            println!("No orders found for market {}.", self.market);
            
            // Try other potential paths
            println!("Trying alternative query paths...");
            let alt_paths = vec![
                format!("orderbook/{}", self.market),
                format!("orderbook/market/{}", self.market),
                format!("state/orderbook/{}", self.market),
                format!("app/orderbook/{}", self.market)
            ];
            
            for alt_path in alt_paths {
                println!("Trying path: {}", alt_path);
                match sequencer_client
                    .abci_query(Some(alt_path.clone()), vec![], Some(0u32.into()), false)
                    .await {
                        Ok(alt_response) => {
                            if !alt_response.value.is_empty() {
                                println!("Got response from {}: {}", alt_path, 
                                    String::from_utf8_lossy(&alt_response.value));
                            } else {
                                println!("Empty response from {}.", alt_path);
                            }
                        },
                        Err(e) => {
                            println!("Error querying {}: {}", alt_path, e);
                        }
                    }
            }
            
            return Ok(());
        }
        
        // Try to parse as JSON to pretty-print
        match serde_json::from_str::<serde_json::Value>(&orderbook_json) {
            Ok(json_value) => {
                println!("Orderbook for market {}:", self.market);
                println!("{}", serde_json::to_string_pretty(&json_value)?);
            },
            Err(e) => {
                println!("Failed to parse response as JSON: {}", e);
                println!("Raw response: {}", orderbook_json);
            }
        }
        
        Ok(())
    }
}