// This script helps create base64-encoded transactions for orderbook testing
// You'll need node.js to run this

const crypto = require('crypto');

// Sample private key - replace with your actual key
const PRIVATE_KEY = 'REPLACE_WITH_YOUR_PRIVATE_KEY';

// Create a market transaction
function createMarketTransaction() {
  const createMarketAction = {
    market: "BTC/USD",
    base_asset: "BTC",
    quote_asset: "USD",
    tick_size: { value: "0.01" },
    lot_size: { value: "0.001" },
    fee_asset: "ASTRIA",
  };
  
  const transaction = {
    nonce: 1, // Get this from sequencer API
    chain_id: "astria",
    actions: [
      {
        value: {
          create_market: createMarketAction
        }
      }
    ]
  };
  
  // This is just a placeholder - actual implementation would need to:
  // 1. Convert to protobuf binary format
  // 2. Sign with private key
  // 3. Base64 encode
  
  console.log("PLACEHOLDER - Transaction would be created and signed here");
  console.log("Transaction details:", JSON.stringify(transaction, null, 2));
  
  return "BASE64_ENCODED_TRANSACTION_WOULD_GO_HERE";
}

// Create an order transaction
function createOrderTransaction() {
  const createOrderAction = {
    market: "BTC/USD",
    side: 0, // 0 = Buy, 1 = Sell
    type: 0, // 0 = Limit, 1 = Market
    price: { value: "35000.00" }, 
    quantity: { value: "0.1" },
    time_in_force: 0, // 0 = GTC, 1 = IOC, 2 = FOK
    fee_asset: "ASTRIA",
  };
  
  const transaction = {
    nonce: 2, // Get this from sequencer API
    chain_id: "astria",
    actions: [
      {
        value: {
          create_order: createOrderAction
        }
      }
    ]
  };
  
  console.log("PLACEHOLDER - Transaction would be created and signed here");
  console.log("Transaction details:", JSON.stringify(transaction, null, 2));
  
  return "BASE64_ENCODED_TRANSACTION_WOULD_GO_HERE";
}

// Cancel an order transaction
function cancelOrderTransaction(orderId) {
  const cancelOrderAction = {
    order_id: orderId,
    fee_asset: "ASTRIA",
  };
  
  const transaction = {
    nonce: 3, // Get this from sequencer API
    chain_id: "astria",
    actions: [
      {
        value: {
          cancel_order: cancelOrderAction
        }
      }
    ]
  };
  
  console.log("PLACEHOLDER - Transaction would be created and signed here");
  console.log("Transaction details:", JSON.stringify(transaction, null, 2));
  
  return "BASE64_ENCODED_TRANSACTION_WOULD_GO_HERE";
}

// Call the functions
console.log("Creating market transaction:");
createMarketTransaction();

console.log("\nCreating order transaction:");
createOrderTransaction();

console.log("\nCanceling order transaction:");
cancelOrderTransaction("order123");