#!/bin/bash

# Test script for interacting with the orderbook functionality
# Replace the URL with your actual sequencer node URL
SEQUENCER_URL="http://localhost:26657"

# Colors for better output
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${GREEN}Testing Orderbook Functionality${NC}"
echo -e "=============================="

# 1. Query available markets
echo -e "\n${CYAN}Querying available markets...${NC}"
curl -X POST "$SEQUENCER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "abci_query",
    "params": {
      "path": "orderbook/markets",
      "data": "",
      "height": "0",
      "prove": false
    }
  }'

# 2. Create a market
# This requires a properly signed and encoded transaction
echo -e "\n\n${CYAN}To create a market, you would run:${NC}"
echo "curl -X POST \"$SEQUENCER_URL\" \\"
echo "  -H \"Content-Type: application/json\" \\"
echo "  -d '{"
echo "    \"jsonrpc\": \"2.0\","
echo "    \"id\": 2,"
echo "    \"method\": \"broadcast_tx_sync\","
echo "    \"params\": {"
echo "      \"tx\": \"BASE64_ENCODED_TX_TO_CREATE_MARKET\""
echo "    }"
echo "  }'"

# 3. Query a specific market's orderbook
echo -e "\n\n${CYAN}To query a market's orderbook, you would run:${NC}"
echo "curl -X POST \"$SEQUENCER_URL\" \\"
echo "  -H \"Content-Type: application/json\" \\"
echo "  -d '{"
echo "    \"jsonrpc\": \"2.0\","
echo "    \"id\": 3,"
echo "    \"method\": \"abci_query\","
echo "    \"params\": {"
echo "      \"path\": \"orderbook/orders/BTC/USD\","
echo "      \"data\": \"\","
echo "      \"height\": \"0\","
echo "      \"prove\": false"
echo "    }"
echo "  }'"

# 4. Create a buy order
# This requires a properly signed and encoded transaction
echo -e "\n\n${CYAN}To create a buy order, you would run:${NC}"
echo "curl -X POST \"$SEQUENCER_URL\" \\"
echo "  -H \"Content-Type: application/json\" \\"
echo "  -d '{"
echo "    \"jsonrpc\": \"2.0\","
echo "    \"id\": 4,"
echo "    \"method\": \"broadcast_tx_sync\","
echo "    \"params\": {"
echo "      \"tx\": \"BASE64_ENCODED_TX_TO_CREATE_BUY_ORDER\""
echo "    }"
echo "  }'"

# 5. Cancel an order
# This requires a properly signed and encoded transaction
echo -e "\n\n${CYAN}To cancel an order, you would run:${NC}"
echo "curl -X POST \"$SEQUENCER_URL\" \\"
echo "  -H \"Content-Type: application/json\" \\"
echo "  -d '{"
echo "    \"jsonrpc\": \"2.0\","
echo "    \"id\": 5,"
echo "    \"method\": \"broadcast_tx_sync\","
echo "    \"params\": {"
echo "      \"tx\": \"BASE64_ENCODED_TX_TO_CANCEL_ORDER\""
echo "    }"
echo "  }'"

echo -e "\n\n${GREEN}Test script completed${NC}"
echo -e "Use create-orderbook-transaction.js to create the base64-encoded transactions needed for the above commands."
echo -e "You'll need to modify the script with your private key and other details."