# Astria Orderbook Component

This component implements a decentralized order book for the Astria sequencer. It allows users to create markets, place orders, and execute trades on-chain.

## Features

- **Market Creation**: Create and manage trading markets with configurable parameters
- **Order Management**: Submit, cancel, and query orders
- **Trade Execution**: Matching engine for executing compatible orders
- **Price Discovery**: Market and limit orders with various time-in-force options
- **Querying**: APIs for retrieving orderbook state, markets, and trade history

## Implementation

The orderbook component is implemented as a standard Astria sequencer component, following the same patterns as other components:

1. It implements the `Component` trait
2. It uses state extension traits (`StateReadExt`, `StateWriteExt`) to read from and write to state
3. It defines `CheckedAction` implementations for various actions
4. It provides query handlers for accessing state via ABCI

## Data Types

The component uses the following core data types:

- **Market**: Represents a trading pair with configuration parameters
- **Order**: A buy or sell order with price, quantity, and execution parameters
- **Trade**: A record of a completed trade between two orders
- **Orderbook**: The current state of all orders for a specific market

## Future Improvements

The current implementation is a simplified version that needs the following improvements:

1. **Protocol Buffer Integration**: Integrate with the full protobuf definitions
2. **Proper Fee Handling**: More sophisticated fee mechanics for order placement and execution
3. **Better Price/Time Priority**: Enhanced order matching algorithm with proper price/time priority
4. **Scalability Optimizations**: Performance improvements for high-frequency markets
5. **Risk Management**: Position limits, margin requirements, etc.
6. **Market Maker Incentives**: Special incentives for liquidity providers

## Usage

To use the orderbook component in your Astria implementation:

1. Include the component in your application setup
2. Create markets using the `CreateMarket` action
3. Submit orders using the `CreateOrder` action
4. Cancel orders using the `CancelOrder` action
5. Query the orderbook state to see current orders and trades

## API Reference

### Actions
- `CreateMarket`: Create a new trading market
- `CreateOrder`: Submit a new order to the orderbook
- `CancelOrder`: Cancel an existing order
- `UpdateMarket`: Update market parameters

### Queries
- `orderbook/markets`: List all available markets
- `orderbook/{market}`: Get full orderbook state for a market
- `orderbook/depth/{market}`: Get aggregated orderbook depth by price level
- `orderbook/orders/owner/{owner}`: Get all orders for a specific owner
- `orderbook/orders/market/{market}/{side}`: Get orders for a market with optional side filter
- `orderbook/order/{order_id}`: Get details of a specific order
- `orderbook/trades/{market}/{limit}`: Get trade history for a market
- `orderbook/market_params/{market}`: Get parameters for a specific market

## License

Same as the Astria project.