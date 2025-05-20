# Testing the Orderbook in Astria

Since the orderbook functionality is still in development, we'll use the following approaches to test it:

## Using the Sequencer RPC API Directly

For testing orderbook functionality, you can use curl to interact with the sequencer RPC API:

### 1. Creating a Market

```bash
curl -X POST "http://localhost:26657" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "broadcast_tx_sync", 
    "params": {
      "tx": "BASE64_ENCODED_TRANSACTION"
    }
  }'
```

The transaction must be a base64-encoded transaction containing a CreateMarket action.

### 2. Query Available Markets

```bash
curl -X POST "http://localhost:26657" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "abci_query",
    "params": {
      "path": "orderbook/markets",
      "data": "",
      "height": "0",
      "prove": false
    }
  }'
```

### 3. Place an Order

```bash
curl -X POST "http://localhost:26657" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "broadcast_tx_sync",
    "params": {
      "tx": "BASE64_ENCODED_TRANSACTION"
    }
  }'
```

The transaction must be a base64-encoded transaction containing a CreateOrder action.

### 4. Query the Orderbook

```bash
curl -X POST "http://localhost:26657" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "abci_query",
    "params": {
      "path": "orderbook/orders/YOUR_MARKET_ID",
      "data": "",
      "height": "0",
      "prove": false
    }
  }'
```

Replace `YOUR_MARKET_ID` with the identifier of your market (e.g., "BTC/USD").

## Funding Requirements

For using the orderbook functionality, you need:

1. A funded account with ASTRIA tokens for paying transaction fees
2. Sufficient balances in the trading assets:
   - Base asset (for sell orders)
   - Quote asset (for buy orders)

## Implementation Status

The orderbook functionality in Astria is still in development. The matching engine may not be fully functional yet, but the basic structure for creating markets and placing orders is in place.

The actual implementation contains placeholder functions in some areas, particularly in the matching engine, but the transaction processing and query endpoints should work for testing purposes.

## Next Steps

As the orderbook functionality continues to be developed, the CLI will be updated to include dedicated commands for interacting with it. Until then, use the approach above for testing.