#\!/bin/bash

echo "Testing Orderbook Functionality"

# 1. Create a market
echo "Creating market BTC/USD..."
SEQUENCER_URL="http://localhost:26657"
PRIVATE_KEY="YOUR_PRIVATE_KEY_HERE"

# Use curl to submit a transaction to create a market
curl -X POST "$SEQUENCER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "broadcast_tx_sync",
    "params": {
      "tx": "BASE64_ENCODED_TX_TO_CREATE_MARKET"
    }
  }'

echo "Waiting for transaction to be included in a block..."
sleep 5

# 2. Query available markets
echo "Querying available markets..."
curl -X POST "$SEQUENCER_URL" \
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

# 3. Create a buy order
echo "Creating buy order for BTC/USD..."
curl -X POST "$SEQUENCER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "broadcast_tx_sync",
    "params": {
      "tx": "BASE64_ENCODED_TX_TO_CREATE_BUY_ORDER"
    }
  }'

echo "Waiting for transaction to be included in a block..."
sleep 5

# 4. Create a sell order
echo "Creating sell order for BTC/USD..."
curl -X POST "$SEQUENCER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "broadcast_tx_sync",
    "params": {
      "tx": "BASE64_ENCODED_TX_TO_CREATE_SELL_ORDER"
    }
  }'

echo "Waiting for transaction to be included in a block..."
sleep 5

# 5. Query the orderbook
echo "Querying orderbook for BTC/USD..."
curl -X POST "$SEQUENCER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 5,
    "method": "abci_query",
    "params": {
      "path": "orderbook/orders/BTC/USD",
      "data": "",
      "height": "0",
      "prove": false
    }
  }'

echo "Orderbook testing completed\!"
