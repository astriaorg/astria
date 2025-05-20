use astria_core::{
    crypto::SigningKey,
    generated::astria::protocol::{
        orderbook::v1 as orderbook_proto,
        transaction::v1::{
            action::Value, Action, CreateMarket, CreateOrder, CancelOrder,
        },
    },
    primitive::v1::Address,
    protocol::transaction::v1::TransactionBody,
};
use astria_sequencer_client::{
    Client as _,
    HttpClient, 
    SequencerClientExt as _,
};
use clap::{Parser, Subcommand};
use color_eyre::eyre::{self, WrapErr as _};
use pbjson_types::Decimal;
use serde_json;
use url::Url;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Sequencer URL to connect to
    #[arg(short, long, default_value = "http://localhost:26657")]
    sequencer_url: String,

    /// Private key to use for transactions (hex-encoded)
    #[arg(short, long)]
    private_key: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new market
    CreateMarket {
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
        fee_asset: String,
    },
    /// Create a new order
    CreateOrder {
        /// Market identifier, e.g., BTC/USD
        market: String,
        /// Order side (buy or sell)
        #[arg(value_enum)]
        side: OrderSideArg,
        /// Order type (limit or market)
        #[arg(value_enum)]
        order_type: OrderTypeArg,
        /// Order price (required for limit orders)
        price: String,
        /// Order quantity
        quantity: String,
        /// Time in force
        #[arg(value_enum)]
        time_in_force: TimeInForceArg,
        /// Asset to pay fees in
        fee_asset: String,
    },
    /// Cancel an existing order
    CancelOrder {
        /// Order ID to cancel
        order_id: String,
        /// Asset to pay fees in
        fee_asset: String,
    },
    /// List available markets
    ListMarkets,
    /// View orderbook for a market
    ViewOrderbook {
        /// Market identifier, e.g., BTC/USD
        market: String,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum OrderSideArg {
    Buy,
    Sell,
}

impl From<OrderSideArg> for i32 {
    fn from(value: OrderSideArg) -> Self {
        match value {
            OrderSideArg::Buy => 0,  // OrderSide::Buy
            OrderSideArg::Sell => 1, // OrderSide::Sell
        }
    }
}

#[derive(clap::ValueEnum, Clone)]
enum OrderTypeArg {
    Limit,
    Market,
}

impl From<OrderTypeArg> for i32 {
    fn from(value: OrderTypeArg) -> Self {
        match value {
            OrderTypeArg::Limit => 0,  // OrderType::Limit
            OrderTypeArg::Market => 1, // OrderType::Market
        }
    }
}

#[derive(clap::ValueEnum, Clone)]
enum TimeInForceArg {
    Gtc,
    Ioc,
    Fok,
}

impl From<TimeInForceArg> for i32 {
    fn from(value: TimeInForceArg) -> Self {
        match value {
            TimeInForceArg::Gtc => 0, // OrderTimeInForce::GTC
            TimeInForceArg::Ioc => 1, // OrderTimeInForce::IOC
            TimeInForceArg::Fok => 2, // OrderTimeInForce::FOK
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::CreateMarket { 
            market, 
            base_asset, 
            quote_asset, 
            tick_size, 
            lot_size, 
            fee_asset 
        } => {
            create_market(
                &cli.sequencer_url, 
                &cli.private_key, 
                market, 
                base_asset, 
                quote_asset, 
                tick_size, 
                lot_size, 
                fee_asset
            ).await
        },
        Commands::CreateOrder { 
            market, 
            side, 
            order_type, 
            price, 
            quantity, 
            time_in_force, 
            fee_asset 
        } => {
            create_order(
                &cli.sequencer_url, 
                &cli.private_key, 
                market, 
                side, 
                order_type, 
                price, 
                quantity, 
                time_in_force, 
                fee_asset
            ).await
        },
        Commands::CancelOrder { 
            order_id, 
            fee_asset 
        } => {
            cancel_order(
                &cli.sequencer_url, 
                &cli.private_key, 
                order_id, 
                fee_asset
            ).await
        },
        Commands::ListMarkets => {
            list_markets(&cli.sequencer_url).await
        },
        Commands::ViewOrderbook { 
            market 
        } => {
            view_orderbook(&cli.sequencer_url, market).await
        },
    }
}

async fn create_market(
    sequencer_url: &str,
    private_key: &str,
    market: String,
    base_asset: String,
    quote_asset: String,
    tick_size: String,
    lot_size: String,
    fee_asset: String,
) -> eyre::Result<()> {
    // Convert tick_size and lot_size to Decimal
    let tick_size_decimal = Decimal { value: tick_size };
    let lot_size_decimal = Decimal { value: lot_size };

    // Create action
    let create_market = CreateMarket {
        market,
        base_asset,
        quote_asset,
        tick_size: Some(tick_size_decimal),
        lot_size: Some(lot_size_decimal),
        fee_asset,
    };

    // Submit transaction
    let action = Action {
        value: Some(Value::CreateMarket(create_market)),
    };
    
    let result = submit_transaction(sequencer_url, private_key, action).await?;
    println!("Market created successfully! Transaction hash: {}", result.hash);
    
    Ok(())
}

async fn create_order(
    sequencer_url: &str,
    private_key: &str,
    market: String,
    side: OrderSideArg,
    order_type: OrderTypeArg,
    price: String,
    quantity: String,
    time_in_force: TimeInForceArg,
    fee_asset: String,
) -> eyre::Result<()> {
    // Convert price and quantity to Decimal
    let price_decimal = Decimal { value: price };
    let quantity_decimal = Decimal { value: quantity };

    // Create action
    let create_order = CreateOrder {
        market,
        side: side.into(),
        r#type: order_type.into(),
        price: Some(price_decimal),
        quantity: Some(quantity_decimal),
        time_in_force: time_in_force.into(),
        fee_asset,
    };

    // Submit transaction
    let action = Action {
        value: Some(Value::CreateOrder(create_order)),
    };
    
    let result = submit_transaction(sequencer_url, private_key, action).await?;
    println!("Order created successfully! Transaction hash: {}", result.hash);
    
    Ok(())
}

async fn cancel_order(
    sequencer_url: &str,
    private_key: &str,
    order_id: String,
    fee_asset: String,
) -> eyre::Result<()> {
    // Create action
    let cancel_order = CancelOrder {
        order_id,
        fee_asset,
    };

    // Submit transaction
    let action = Action {
        value: Some(Value::CancelOrder(cancel_order)),
    };
    
    let result = submit_transaction(sequencer_url, private_key, action).await?;
    println!("Order cancelled successfully! Transaction hash: {}", result.hash);
    
    Ok(())
}

async fn list_markets(sequencer_url: &str) -> eyre::Result<()> {
    let sequencer_client = HttpClient::new(sequencer_url)
        .wrap_err("failed constructing http sequencer client")?;
    
    let response = sequencer_client
        .abci_query(Some("orderbook/markets".to_string()), vec![], Some(0u32.into()), false)
        .await
        .wrap_err("failed to query markets")?;
    
    // The response is a JSON-encoded array of Market objects
    let markets_json = String::from_utf8(response.value.to_vec())
        .wrap_err("failed to convert response to string")?;
    
    let markets: Vec<orderbook_proto::Market> = 
        serde_json::from_str(&markets_json)
        .wrap_err("failed to parse markets from JSON")?;
    
    println!("Available Markets:");
    for market in markets {
        // Safely display tick size and lot size values
        let tick_size = market.tick_size
            .map(|d| d.value.clone())
            .unwrap_or_else(|| "N/A".to_string());
            
        let lot_size = market.lot_size
            .map(|d| d.value.clone())
            .unwrap_or_else(|| "N/A".to_string());
            
        println!(
            "  {}: {}/{} (tick: {}, lot: {})",
            market.market,
            market.base_asset,
            market.quote_asset,
            tick_size,
            lot_size
        );
    }
    
    Ok(())
}

async fn view_orderbook(sequencer_url: &str, market: String) -> eyre::Result<()> {
    let sequencer_client = HttpClient::new(sequencer_url)
        .wrap_err("failed constructing http sequencer client")?;
    
    let path = format!("orderbook/orders/{}", market);
    let response = sequencer_client
        .abci_query(Some(path), vec![], Some(0u32.into()), false)
        .await
        .wrap_err("failed to query orderbook")?;
    
    // The response is a JSON-encoded Orderbook object
    let orderbook_json = String::from_utf8(response.value.to_vec())
        .wrap_err("failed to convert response to string")?;
    
    let orderbook: orderbook_proto::Orderbook = 
        serde_json::from_str(&orderbook_json)
        .wrap_err("failed to parse orderbook from JSON")?;
    
    println!("Orderbook for market {}:", market);
    
    println!("Bids:");
    for level in orderbook.bids {
        // Convert proto decimal to string for display
        let quantity = level.quantity
            .map(|d| d.value.clone())
            .unwrap_or_else(|| "N/A".to_string());
            
        let price = level.price
            .map(|d| d.value.clone())
            .unwrap_or_else(|| "N/A".to_string());
            
        println!(
            "  {} @ {} ({} orders)",
            quantity,
            price,
            level.order_count
        );
    }
    
    println!("Asks:");
    for level in orderbook.asks {
        // Convert proto decimal to string for display
        let quantity = level.quantity
            .map(|d| d.value.clone())
            .unwrap_or_else(|| "N/A".to_string());
            
        let price = level.price
            .map(|d| d.value.clone())
            .unwrap_or_else(|| "N/A".to_string());
            
        println!(
            "  {} @ {} ({} orders)",
            quantity,
            price,
            level.order_count
        );
    }
    
    Ok(())
}

async fn submit_transaction(
    sequencer_url: &str,
    private_key: &str,
    action: Action,
) -> eyre::Result<astria_sequencer_client::tendermint_rpc::endpoint::tx::Response> {
    // Create HTTP client
    let sequencer_client = HttpClient::new(sequencer_url)
        .wrap_err("failed to create HTTP client")?;

    // Create signing key from private key
    let private_key_bytes = hex::decode(private_key)
        .wrap_err("failed to decode private key")?;
    let signing_key = SigningKey::try_from(private_key_bytes.as_slice())
        .wrap_err("invalid private key")?;

    // Get address from signing key
    let from_address = Address::builder()
        .array(*signing_key.verification_key().address_bytes())
        .prefix("astria")
        .try_build()
        .wrap_err("failed to create address from private key")?;

    println!("Sending transaction from address: {from_address}");

    // Get latest nonce
    let nonce_res = sequencer_client
        .get_latest_nonce(from_address)
        .await
        .wrap_err("failed to get nonce")?;

    // Create and sign transaction
    let tx = TransactionBody::builder()
        .nonce(nonce_res.nonce)
        .chain_id("astria".to_string())
        .actions(vec![action])
        .try_build()
        .wrap_err("failed to construct transaction")?
        .sign(&signing_key);

    // Submit transaction
    let res = sequencer_client
        .submit_transaction_sync(tx)
        .await
        .wrap_err("failed to submit transaction")?;

    if !res.code.is_ok() {
        return Err(eyre::eyre!("failed to check tx: {}", res.log));
    }

    // Wait for transaction inclusion
    let tx_response = sequencer_client.wait_for_tx_inclusion(res.hash).await;

    if !tx_response.tx_result.code.is_ok() {
        return Err(eyre::eyre!("failed to execute tx: {}", tx_response.tx_result.log));
    }

    Ok(tx_response)
}