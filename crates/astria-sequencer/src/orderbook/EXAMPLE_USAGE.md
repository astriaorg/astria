# Orderbook Component Example Usage

This document provides examples of how to use the orderbook component once it's fully integrated into the Astria sequencer.

## Creating a Market

To create a new trading market, you'll need to submit a `CreateMarket` transaction:

```rust
use astria_core::protocol::orderbook::v1::CreateMarket;
use astria_core::primitive::v1::Uint128;

// Create the action
let create_market_action = CreateMarket {
    market: "BTC/USD".to_string(),
    base_asset: "BTC".to_string(),
    quote_asset: "USD".to_string(),
    tick_size: Uint128::from(1), // $0.01 minimum price increment
    lot_size: Uint128::from(100000), // 0.00001 BTC minimum quantity
    fee_asset: "ASTRIA".to_string(), // Pay fees in ASTRIA tokens
};

// Create the transaction
let tx = Transaction::new()
    .with_action(Action::OrderbookCreateMarket(create_market_action))
    .with_nonce(nonce)
    .with_gas_limit(100000)
    .sign(private_key);

// Submit the transaction
client.submit_transaction(tx).await?;
```

## Placing an Order

To place a new order in the orderbook, you'll need to submit a `CreateOrder` transaction:

```rust
use astria_core::protocol::orderbook::v1::{CreateOrder, OrderSide, OrderType, OrderTimeInForce};
use astria_core::primitive::v1::Uint128;

// Create a limit buy order
let create_order_action = CreateOrder {
    market: "BTC/USD".to_string(),
    side: OrderSide::ORDER_SIDE_BUY,
    type_: OrderType::ORDER_TYPE_LIMIT,
    price: Uint128::from(3500000), // $35,000.00
    quantity: Uint128::from(10000000), // 0.1 BTC
    time_in_force: OrderTimeInForce::ORDER_TIME_IN_FORCE_GTC,
    fee_asset: "ASTRIA".to_string(),
};

// Create the transaction
let tx = Transaction::new()
    .with_action(Action::OrderbookCreateOrder(create_order_action))
    .with_nonce(nonce)
    .with_gas_limit(100000)
    .sign(private_key);

// Submit the transaction
client.submit_transaction(tx).await?;
```

## Canceling an Order

To cancel an existing order, you'll need to submit a `CancelOrder` transaction:

```rust
use astria_core::protocol::orderbook::v1::CancelOrder;

// Create a cancel order action
let cancel_order_action = CancelOrder {
    order_id: "order123".to_string(), // The ID of the order to cancel
    fee_asset: "ASTRIA".to_string(),
};

// Create the transaction
let tx = Transaction::new()
    .with_action(Action::OrderbookCancelOrder(cancel_order_action))
    .with_nonce(nonce)
    .with_gas_limit(50000)
    .sign(private_key);

// Submit the transaction
client.submit_transaction(tx).await?;
```

## Querying the Orderbook

There are two ways to query the orderbook: full orderbook or orderbook depth.

### Full Orderbook

To query the complete state of a market's orderbook:

```rust
use astria_core::protocol::orderbook::v1::Orderbook;

// Query the orderbook for a specific market
let orderbook: Orderbook = client
    .query("orderbook/BTC/USD")
    .await?;

// Display the top 5 bids and asks
println!("Market: {}", orderbook.market);
println!("Top 5 Bids:");
for (i, bid) in orderbook.bids.iter().take(5).enumerate() {
    println!("  {}. {} @ {} ({} orders)", 
        i+1, bid.quantity, bid.price, bid.order_count);
}

println!("Top 5 Asks:");
for (i, ask) in orderbook.asks.iter().take(5).enumerate() {
    println!("  {}. {} @ {} ({} orders)", 
        i+1, ask.quantity, ask.price, ask.order_count);
}
```

### Orderbook Depth

For a more efficient aggregated view of the orderbook by price level:

```rust
use astria_core::protocol::orderbook::v1::OrderbookDepth;

// Query the orderbook depth with default levels (10)
let depth: OrderbookDepth = client
    .query("orderbook/depth/BTC/USD")
    .await?;

// Query with a specific number of levels
let depth_20: OrderbookDepth = client
    .query("orderbook/depth/BTC/USD?levels=20")
    .await?;

// Display the price levels
println!("Market: {}", depth.market);
println!("Bids:");
for (i, bid) in depth.bids.iter().enumerate() {
    println!("  {}. {} @ {}", i+1, bid.quantity, bid.price);
}

println!("Asks:");
for (i, ask) in depth.asks.iter().enumerate() {
    println!("  {}. {} @ {}", i+1, ask.quantity, ask.price);
}
```

## Querying Orders by Owner

To query all orders placed by a specific owner:

```rust
use astria_core::protocol::orderbook::v1::Order;

// Query all orders for a specific owner
let owner_address = "astria1abc123...";
let orders: Vec<Order> = client
    .query(format!("orderbook/orders/owner/{}", owner_address))
    .await?;

// Display the orders
println!("Orders for {}:", owner_address);
for (i, order) in orders.iter().enumerate() {
    println!("  {}. {} {} {} @ {} ({}%)", 
        i+1, 
        order.remaining_quantity, 
        order.market,
        if order.side == OrderSide::ORDER_SIDE_BUY { "BUY" } else { "SELL" },
        order.price,
        (order.remaining_quantity.as_str().parse::<f64>().unwrap() / 
         order.quantity.as_str().parse::<f64>().unwrap() * 100.0).round()
    );
}
```

## Querying Trade History

To query the trade history for a specific market:

```rust
use astria_core::protocol::orderbook::v1::OrderMatch;

// Query recent trades for a specific market
let market = "BTC/USD";
let limit = 10; // Number of trades to return
let trades: Vec<OrderMatch> = client
    .query(format!("orderbook/trades/{}/{}", market, limit))
    .await?;

// Display the trades
println!("Recent trades for {}:", market);
for (i, trade) in trades.iter().enumerate() {
    println!("  {}. {} {} @ {} ({})", 
        i+1, 
        trade.quantity, 
        market,
        trade.price,
        trade.timestamp
    );
}
```

## Tracking Events

To track orderbook events in real-time:

```rust
// Subscribe to orderbook events
let mut event_subscription = client.subscribe_events([
    "orderbook_create_order",
    "orderbook_cancel_order",
    "orderbook_trade",
]).await?;

// Process events as they come in
while let Some(event) = event_subscription.next().await {
    match event.event_type {
        "orderbook_create_order" => {
            let order_id = event.attributes.get("order_id").unwrap();
            let market = event.attributes.get("market").unwrap();
            let side = event.attributes.get("side").unwrap();
            println!("New order: {} {} on {}", side, order_id, market);
        },
        "orderbook_cancel_order" => {
            let order_id = event.attributes.get("order_id").unwrap();
            println!("Order cancelled: {}", order_id);
        },
        "orderbook_trade" => {
            let market = event.attributes.get("market").unwrap();
            let price = event.attributes.get("price").unwrap();
            let quantity = event.attributes.get("quantity").unwrap();
            println!("Trade: {} {} @ {}", quantity, market, price);
        },
        _ => {}
    }
}
```

These examples demonstrate the basic usage patterns for interacting with the orderbook component. The actual implementation details might vary based on the final API design and client libraries available.