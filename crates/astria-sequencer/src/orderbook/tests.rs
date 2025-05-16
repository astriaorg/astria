#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use astria_core::{
        primitive::v1::Address,
        protocol::orderbook::v1::{Order, OrderSide, OrderTimeInForce, OrderType},
    };
    use cnidarium::{StateDelta, Storage};
    use tendermint::abci::{request, Event};
    use uuid::Uuid;

    use crate::{
        orderbook::{
            component::{CheckedCreateMarket, CheckedCreateOrder, OrderbookComponent},
            matching_engine::MatchingEngine,
            state_ext::{StateReadExt, StateWriteExt},
        },
        checked_actions::{CheckedAction, CheckedActionError},
    };

    #[tokio::test]
    async fn test_create_and_query_market() {
        // Create an in-memory storage
        let storage = Storage::new_ephemeral().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // Create a new market
        let market_id = "BTC/USD".to_string();
        let base_asset = "BTC".to_string();
        let quote_asset = "USD".to_string();
        let tick_size = "0.01".to_string();
        let lot_size = "0.001".to_string();
        let fee_asset = "USD".to_string();

        // Create a sender address
        let sender = Address::from_str("astria1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq48uyvm").unwrap();

        // Create a CheckedCreateMarket
        let checked_market = CheckedCreateMarket {
            sender: sender.clone(),
            market: market_id.clone(),
            base_asset: base_asset.clone(),
            quote_asset: quote_asset.clone(),
            tick_size: tick_size.clone(),
            lot_size: lot_size.clone(),
            fee_asset: fee_asset.clone(),
        };

        // Execute the action
        let result = checked_market.execute(&mut state);
        assert!(result.is_ok(), "Failed to create market: {:?}", result.err());

        // Verify the market was created
        assert!(state.market_exists(&market_id), "Market was not created");

        // Get and check market params
        let params = state.get_market_params(&market_id).unwrap();
        assert_eq!(params.base_asset, base_asset);
        assert_eq!(params.quote_asset, quote_asset);
        assert_eq!(params.tick_size, tick_size);
        assert_eq!(params.lot_size, lot_size);
        assert_eq!(params.paused, false);

        // Create an order
        let checked_order = CheckedCreateOrder {
            sender: sender.clone(),
            market: market_id.clone(),
            side: OrderSide::ORDER_SIDE_BUY,
            order_type: OrderType::ORDER_TYPE_LIMIT,
            price: "40000.00".to_string(),
            quantity: "0.5".to_string(),
            time_in_force: OrderTimeInForce::ORDER_TIME_IN_FORCE_GTC,
            fee_asset: fee_asset.clone(),
        };

        // Execute the order creation
        let result = checked_order.execute(&mut state);
        assert!(result.is_ok(), "Failed to create order: {:?}", result.err());

        // Check that order exists in orderbook
        let orders = state.get_market_orders(&market_id, Some(OrderSide::ORDER_SIDE_BUY)).collect::<Vec<_>>();
        assert_eq!(orders.len(), 1, "Order was not created");

        let order = &orders[0];
        assert_eq!(order.market, market_id);
        assert_eq!(order.side, OrderSide::ORDER_SIDE_BUY);
        assert_eq!(order.price, "40000.00");
        assert_eq!(order.quantity, "0.5");
        assert_eq!(order.remaining_quantity, "0.5");
        assert_eq!(order.owner, sender.to_string());

        // Test the orderbook state
        let orderbook = state.get_orderbook(&market_id);
        assert_eq!(orderbook.market, market_id);
        assert_eq!(orderbook.bids.len(), 1, "Bid was not added to orderbook");
        assert_eq!(orderbook.asks.len(), 0, "There should be no asks");

        let bid = &orderbook.bids[0];
        assert_eq!(bid.price, "40000.00");
        assert_eq!(bid.quantity, "0.5");
        assert_eq!(bid.order_count, 1);
    }

    #[tokio::test]
    async fn test_component_initialization() {
        // Initialize the component
        let component = OrderbookComponent::default();
        
        // Create an in-memory storage
        let storage = Storage::new_ephemeral().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);
        
        // Create a request
        let request = request::InitChain::default();
        
        // Call init_chain
        let result = component.init_chain(&mut state, &request);
        assert!(result.is_ok(), "Failed to initialize component: {:?}", result);
    }

    #[tokio::test]
    async fn test_matching_engine() {
        // Create an in-memory storage
        let storage = Storage::new_ephemeral().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // Create a market first
        let market_id = "BTC/USD".to_string();
        let base_asset = "BTC".to_string();
        let quote_asset = "USD".to_string();
        let tick_size = "0.01".to_string();
        let lot_size = "0.001".to_string();
        let sender = Address::from_str("astria1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq48uyvm").unwrap();

        // Add market to state
        let params = crate::orderbook::state_ext::MarketParams {
            base_asset,
            quote_asset,
            tick_size,
            lot_size,
            paused: false,
        };
        state.add_market(&market_id, params).unwrap();

        // Create buy order
        let buy_order_id = Uuid::new_v4().to_string();
        let buy_order = Order {
            id: buy_order_id.clone(),
            owner: sender.to_string(),
            market: market_id.clone(),
            side: OrderSide::ORDER_SIDE_BUY,
            type_: OrderType::ORDER_TYPE_LIMIT as i32,
            price: "40000.00".to_string(),
            quantity: "1.0".to_string(),
            remaining_quantity: "1.0".to_string(),
            created_at: 1000,
            time_in_force: OrderTimeInForce::ORDER_TIME_IN_FORCE_GTC as i32,
            fee_asset: "USD".to_string(),
        };

        // Add buy order to the book
        state.put_order(buy_order.clone()).unwrap();

        // Create the matching engine
        let engine = MatchingEngine::default();

        // Create a matching sell order
        let sell_order_id = Uuid::new_v4().to_string();
        let sell_order = Order {
            id: sell_order_id.clone(),
            owner: sender.to_string(),
            market: market_id.clone(),
            side: OrderSide::ORDER_SIDE_SELL,
            type_: OrderType::ORDER_TYPE_LIMIT as i32,
            price: "40000.00".to_string(),
            quantity: "0.5".to_string(),
            remaining_quantity: "0.5".to_string(),
            created_at: 2000,
            time_in_force: OrderTimeInForce::ORDER_TIME_IN_FORCE_GTC as i32,
            fee_asset: "USD".to_string(),
        };

        // Process the sell order
        let matches = engine.process_order(&mut state, sell_order.clone()).unwrap();

        // Verify matches
        assert_eq!(matches.len(), 1, "Should have one match");
        let trade_match = &matches[0];
        assert_eq!(trade_match.market, market_id);
        assert_eq!(trade_match.price, "40000.00");
        assert_eq!(trade_match.quantity, "0.5");
        assert_eq!(trade_match.maker_order_id, buy_order_id);
        assert_eq!(trade_match.taker_order_id, sell_order_id);
        assert_eq!(trade_match.taker_side, OrderSide::ORDER_SIDE_SELL);

        // Check updated order quantities
        let updated_buy_order = state.get_order(&buy_order_id).unwrap();
        assert_eq!(updated_buy_order.remaining_quantity, "0.5", "Buy order should have 0.5 remaining");

        let updated_sell_order = state.get_order(&sell_order_id).unwrap();
        assert_eq!(updated_sell_order.remaining_quantity, "0.0", "Sell order should be fully matched");

        // Check the orderbook
        let orderbook = state.get_orderbook(&market_id);
        assert_eq!(orderbook.bids.len(), 1, "Should still have 1 bid");
        assert_eq!(orderbook.asks.len(), 0, "Should have no asks");
        assert_eq!(orderbook.bids[0].quantity, "0.5", "Bid quantity should be 0.5");
    }
}