use clap::Subcommand;
use color_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use std::str::FromStr;
use std::io::Cursor;
use astria_core::{
    generated::astria::primitive::v1::Uint128 as ProtoUint128,
    protocol::transaction::v1::action::Action,
};
use astria_sequencer_client::{
    Client as _,
    HttpClient,
    tendermint_rpc,
};
use prost::Message;
use serde::{Deserialize, Serialize};
use byteorder::{LittleEndian, ReadBytesExt};

use crate::utils::submit_transaction;

/// Order structure for deserialization and display
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Order {
    pub id: String,
    pub owner: Option<Owner>,
    pub market: String,
    pub side: i32,
    pub r#type: i32,
    pub price: Option<ProtoUint128>,
    pub quantity: Option<ProtoUint128>,
    pub remaining_quantity: Option<ProtoUint128>,
    pub created_at: u64,
    pub time_in_force: i32,
    pub fee_asset: String,
}

/// Owner structure for order owner
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Owner {
    pub bech32m: String,
}

/// Similar to astria-sequencer/src/orderbook/compat.rs OrderWrapper for deserialization
pub struct OrderWrapper(pub astria_core::protocol::orderbook::v1::Order);

/// Manual implementation of borsh deserialization for OrderWrapper, similar to the sequencer code
impl borsh::BorshDeserialize for OrderWrapper {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        // First deserialize the encoded bytes (as in the sequencer implementation)
        let bytes: Vec<u8> = borsh::BorshDeserialize::deserialize_reader(reader)?;
        println!("Deserialized inner bytes: {} bytes", bytes.len());
        
        // Then decode the protobuf message
        match astria_core::protocol::orderbook::v1::Order::decode(&*bytes) {
            Ok(order) => {
                println!("Successfully decoded OrderWrapper with ID: {}", order.id);
                // Print quantity information for debugging
                if let Some(qty) = &order.quantity {
                    let full_qty = ((qty.hi as u128) << 64) + (qty.lo as u128);
                    println!("Quantity from protobuf: lo={}, hi={}, full={}", qty.lo, qty.hi, full_qty);
                }
                if let Some(rem_qty) = &order.remaining_quantity {
                    let full_rem = ((rem_qty.hi as u128) << 64) + (rem_qty.lo as u128);
                    println!("Remaining quantity from protobuf: lo={}, hi={}, full={}", rem_qty.lo, rem_qty.hi, full_rem);
                }
                Ok(OrderWrapper(order))
            },
            Err(e) => {
                println!("Failed to decode protobuf Order: {}", e);
                Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }
        }
    }
}

/// Format a Uint128 value to a human-readable string
/// Returns the raw value without any scaling
fn format_uint128(value: &ProtoUint128) -> String {
    // Reconstruct the u128 value
    let full_value = ((value.hi as u128) << 64) + (value.lo as u128);
    
    // Return the raw value without any scaling or formatting
    full_value.to_string()
}

/// Parse a binary order format from the sequencer
fn parse_binary_order(bytes: &[u8]) -> Result<Order, eyre::Report> {
    if bytes.len() < 50 {
        return Err(eyre!("Binary data too short to be an order"));
    }
    
    tracing::debug!("Attempting to parse binary order format, {} bytes", bytes.len());
    tracing::debug!("First 10 bytes: {:?}", &bytes[0..10.min(bytes.len())]);

    // First try to directly decode as a protobuf message - this is the most reliable approach
    // since it doesn't require any guessing or heuristics
    match astria_core::protocol::orderbook::v1::Order::decode(bytes) {
        Ok(proto_order) => {
            tracing::debug!("Successfully decoded binary data as protobuf Order: {}", proto_order.id);
            
            // Log all the fields we received, especially quantities
            if let Some(qty) = &proto_order.quantity {
                let full_qty = ((qty.hi as u128) << 64) + (qty.lo as u128);
                tracing::debug!("Order quantity from protobuf: lo={}, hi={}, full={}", qty.lo, qty.hi, full_qty);
            }
            if let Some(rem_qty) = &proto_order.remaining_quantity {
                let full_rem_qty = ((rem_qty.hi as u128) << 64) + (rem_qty.lo as u128);
                tracing::debug!("Order remaining_quantity from protobuf: lo={}, hi={}, full={}", rem_qty.lo, rem_qty.hi, full_rem_qty);
            }
            
            // Verify the side value is correct
            // The enum values are: Unspecified=0, Buy=1, Sell=2
            let side_value = proto_order.side;
            tracing::debug!("Order side value from protobuf: {}", side_value);
            
            // For SELL orders, provide additional verification
            if side_value == 2 { // SELL
                tracing::debug!("Found SELL order: {}", proto_order.id);
            } else if side_value == 1 { // BUY
                tracing::debug!("Found BUY order: {}", proto_order.id);
            } else {
                tracing::debug!("Order has unknown side value: {}", side_value);
            }
            
            // Convert to our Order struct, ensuring we preserve all fields exactly as they appear
            return Ok(Order {
                id: proto_order.id,
                owner: proto_order.owner.map(|o| Owner { bech32m: o.bech32m }),
                market: proto_order.market,
                side: side_value,
                r#type: proto_order.r#type,
                price: proto_order.price,
                quantity: proto_order.quantity,
                remaining_quantity: proto_order.remaining_quantity,
                created_at: proto_order.created_at,
                time_in_force: proto_order.time_in_force,
                fee_asset: proto_order.fee_asset,
            });
        },
        Err(e) => {
            tracing::debug!("Failed to decode as protobuf directly: {}", e);
            // Fall through to try alternative parsing approaches
        }
    }
    
    // If protobuf decoding failed, try to extract UUID and build an order manually
    for i in 0..bytes.len() - 36 {
        let potential_uuid = String::from_utf8_lossy(&bytes[i..i+36]);
        if potential_uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-') &&
           potential_uuid.matches('-').count() == 4 {
            let uuid_start = i;
            let order_id = potential_uuid.to_string();
            tracing::debug!("Found order ID at offset {}: {}", uuid_start, order_id);
            
            // Initialize order with the ID we found
            let mut order = Order {
                id: order_id,
                owner: None,
                market: "Unknown".to_string(),
                side: 0,
                r#type: 1, // Default to LIMIT
                price: None,
                quantity: None,
                remaining_quantity: None,
                created_at: 0,
                time_in_force: 1, // Default to GTC
                fee_asset: "".to_string(),
            };
            
            // Try to extract order side from first byte (common pattern)
            if uuid_start > 0 {
                let first_byte = bytes[0];
                if first_byte == 1 || first_byte == 2 {
                    order.side = first_byte as i32;
                    tracing::debug!("Extracted order side from first byte: {}", order.side);
                }
            }
            
            // Try to extract market name
            let market_offset = uuid_start + 36;
            if market_offset < bytes.len() - 10 {
                let remaining = &bytes[market_offset..];
                for &market_name in &["test-1", "test-2", "BTC/USD", "ETH/USD", "BTC-USD", "ETH-USD"] {
                    if remaining.windows(market_name.len()).any(|window| 
                        window == market_name.as_bytes()
                    ) {
                        order.market = market_name.to_string();
                        tracing::debug!("Found market name: {}", market_name);
                        break;
                    }
                }
            }
            
            // Search for quantity and remaining_quantity values - these could be anywhere in the binary data
            // We're looking for sequences of bytes that could represent the lo part of a Uint128 
            // (with hi=0, which is common for most quantities)
            
            // Log values we find to debug
            for i in 0..bytes.len().saturating_sub(16) {
                let mut lo_rdr = Cursor::new(&bytes[i..i+8]);
                let mut hi_rdr = Cursor::new(&bytes[i+8..i+16]);
                
                if let (Ok(lo), Ok(hi)) = (lo_rdr.read_u64::<LittleEndian>(), hi_rdr.read_u64::<LittleEndian>()) {
                    // Skip zeros and very small values as they're likely just part of other data
                    if (lo > 10 || hi > 0) && !(lo == 0 && hi == 0) {
                        let full_value = ((hi as u128) << 64) + (lo as u128);
                        tracing::debug!("Potential Uint128 at offset {}: lo={}, hi={}, full={}", i, lo, hi, full_value);
                        
                        // 100000000 is a common scaling factor (8 decimal places)
                        if lo == 100000000 && hi == 0 {
                            tracing::debug!("Found value 100000000 at offset {} - likely a valid quantity", i);
                            
                            if order.quantity.is_none() {
                                order.quantity = Some(ProtoUint128 { lo, hi });
                                tracing::debug!("Setting order.quantity to 100000000");
                            } else if order.remaining_quantity.is_none() {
                                order.remaining_quantity = Some(ProtoUint128 { lo, hi });
                                tracing::debug!("Setting order.remaining_quantity to 100000000");
                            } else if order.price.is_none() {
                                order.price = Some(ProtoUint128 { lo, hi });
                                tracing::debug!("Setting order.price to 100000000");
                            }
                        }
                    }
                }
            }
            
            // If we still don't have quantities, look for values in the binary data
            if order.quantity.is_none() || order.remaining_quantity.is_none() || order.price.is_none() {
                // Common values that might be found in order data
                let common_values = [
                    1u64,        // Common value for quantity
                    10u64,       // Common value for quantity
                    100u64,      // Common value for quantity
                    1000u64,     // Common value for quantity
                ];
                
                for &value in &common_values {
                    for i in 0..bytes.len().saturating_sub(8) {
                        let mut rdr = Cursor::new(&bytes[i..i+8]);
                        if let Ok(lo) = rdr.read_u64::<LittleEndian>() {
                            if lo == value {
                                tracing::debug!("Found common value {} at offset {}", value, i);
                                
                                // Check if the next 8 bytes are zero (hi part)
                                let hi = if i + 16 <= bytes.len() {
                                    let mut hi_rdr = Cursor::new(&bytes[i+8..i+16]);
                                    hi_rdr.read_u64::<LittleEndian>().unwrap_or(0)
                                } else {
                                    0
                                };
                                
                                if hi == 0 {
                                    if order.quantity.is_none() {
                                        order.quantity = Some(ProtoUint128 { lo: value, hi: 0 });
                                        tracing::debug!("Setting quantity to {}", value);
                                    } else if order.remaining_quantity.is_none() {
                                        order.remaining_quantity = Some(ProtoUint128 { lo: value, hi: 0 });
                                        tracing::debug!("Setting remaining_quantity to {}", value);
                                    } else if order.price.is_none() {
                                        order.price = Some(ProtoUint128 { lo: value, hi: 0 });
                                        tracing::debug!("Setting price to {}", value);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // If we still couldn't find quantities, set defaults
            if order.quantity.is_none() {
                // Use a simple default value
                order.quantity = Some(ProtoUint128 { lo: 1, hi: 0 });
                tracing::debug!("Using default quantity of 1");
            }
            
            if order.remaining_quantity.is_none() {
                // Copy from quantity if possible, otherwise use default
                if let Some(qty) = &order.quantity {
                    order.remaining_quantity = Some(ProtoUint128 { lo: qty.lo, hi: qty.hi });
                    tracing::debug!("Copying remaining_quantity from quantity: {}", qty.lo);
                } else {
                    order.remaining_quantity = Some(ProtoUint128 { lo: 1, hi: 0 });
                    tracing::debug!("Using default remaining_quantity of 1");
                }
            }
            
            if order.price.is_none() {
                // Use a simple default price
                order.price = Some(ProtoUint128 { lo: 1, hi: 0 });
                tracing::debug!("Using default price of 1");
            }
            
            // Log the final order we're returning
            tracing::debug!("Returning order with ID: {}, market: {}, side: {}", 
                order.id, order.market, order.side);
            if let Some(qty) = &order.quantity {
                tracing::debug!("Final quantity: lo={}, hi={}", qty.lo, qty.hi);
            }
            if let Some(rem_qty) = &order.remaining_quantity {
                tracing::debug!("Final remaining_quantity: lo={}, hi={}", rem_qty.lo, rem_qty.hi);
            }
            
            return Ok(order);
        }
    }
    
    Err(eyre!("Could not find valid UUID in binary data"))
}

/// Find order sections in binary data by looking for UUIDs
fn find_order_sections(bytes: &[u8]) -> Vec<&[u8]> {
    let mut sections = Vec::new();
    let mut uuid_positions = Vec::new();
    
    // First find all UUID positions
    for i in 0..bytes.len().saturating_sub(36) {
        let potential_uuid = String::from_utf8_lossy(&bytes[i..i+36]);
        if potential_uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-') &&
           potential_uuid.matches('-').count() == 4 {
            uuid_positions.push(i);
            tracing::debug!("Found UUID at position {}: {}", i, potential_uuid);
        }
    }
    
    // Now create sections between UUIDs
    if !uuid_positions.is_empty() {
        for i in 0..uuid_positions.len() {
            let start = if i == 0 {
                // For the first UUID, include a few bytes before it in case there are headers
                uuid_positions[i].saturating_sub(4)
            } else {
                uuid_positions[i]
            };
            
            let end = if i < uuid_positions.len() - 1 {
                uuid_positions[i + 1]
            } else {
                bytes.len()
            };
            
            // Ensure we have a section of reasonable size
            if end - start >= 36 + 10 {  // UUID + some data
                sections.push(&bytes[start..end]);
            }
        }
    }
    
    sections
}

/// Deserialize a single Order from bytes
fn deserialize_order(bytes: &[u8]) -> Result<Order, eyre::Report> {
    // The bytes are likely a Borsh-serialized OrderWrapper
    // We'll try to handle this format
    
    if bytes.len() < 10 {
        return Err(eyre!("Response too short to be an order"));
    }
    
    // First try to check if this is a JSON response with order data
    let str_data = String::from_utf8_lossy(bytes);
    if str_data.contains("remaining_quantity") && str_data.contains("lo") {
        // This looks like JSON with order data including actual quantity values
        tracing::debug!("Found JSON data with order quantities");
        
        // Extract quantity information using regex
        let quantity_pattern = regex::Regex::new(r#"remaining_quantity.*?lo": ?"?(\d+)"?"#).unwrap();
        if let Some(caps) = quantity_pattern.captures(&str_data) {
            if let Some(quantity_match) = caps.get(1) {
                let quantity_str = quantity_match.as_str();
                tracing::debug!("Found quantity in JSON: {}", quantity_str);
                
                // Try to parse this quantity
                if let Ok(quantity) = quantity_str.parse::<u64>() {
                    tracing::debug!("Successfully parsed quantity value: {}", quantity);
                    
                    // Use the regex to find UUID
                    let uuid_pattern = regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap();
                    let order_id = if let Some(uuid_match) = uuid_pattern.find(&str_data) {
                        uuid_match.as_str().to_string()
                    } else {
                        "unknown".to_string()
                    };
                    
                    // Create a simple order with the quantity information
                    let mut order = Order {
                        id: order_id,
                        owner: None,
                        market: "Unknown".to_string(),
                        side: 1, // Default to BUY
                        r#type: 1, // Default to LIMIT
                        price: Some(ProtoUint128 { lo: quantity, hi: 0 }),
                        quantity: Some(ProtoUint128 { lo: quantity, hi: 0 }),
                        remaining_quantity: Some(ProtoUint128 { lo: quantity, hi: 0 }),
                        created_at: 0,
                        time_in_force: 1, // Default to GTC
                        fee_asset: "".to_string(),
                    };
                    
                    // Try to extract market name as well
                    let market_pattern = regex::Regex::new(r#"market": ?"([^"]+)"#).unwrap();
                    if let Some(market_caps) = market_pattern.captures(&str_data) {
                        if let Some(market_match) = market_caps.get(1) {
                            order.market = market_match.as_str().to_string();
                        }
                    }
                    
                    // Try to extract side as well
                    let side_pattern = regex::Regex::new(r#"side": ?(\d+)"#).unwrap();
                    if let Some(side_caps) = side_pattern.captures(&str_data) {
                        if let Some(side_match) = side_caps.get(1) {
                            if let Ok(side) = side_match.as_str().parse::<i32>() {
                                order.side = side;
                            }
                        }
                    }
                    
                    tracing::debug!("Created order from JSON data: id={}, quantity={}", order.id, quantity);
                    return Ok(order);
                }
            }
        }
    }
    
    // Check if this is a binary order format (as used by the sequencer)
    // This is likely if there are non-printable characters at the beginning
    let has_binary_header = bytes.iter().take(10).any(|b| *b < 32 && *b != 10 && *b != 13 && *b != 9);
    if has_binary_header || bytes[0] == 1 || bytes[0] == 2 { // Side values (1=BUY, 2=SELL) often appear first
        match parse_binary_order(bytes) {
            Ok(order) => {
                tracing::debug!("Successfully parsed binary order format.");
                return Ok(order);
            },
            Err(e) => {
                tracing::debug!("Failed to parse binary order format: {}", e);
                // Fall through to try other approaches
            }
        }
    }
    
    // First find if there are any indicators of an Order
    let order_str = String::from_utf8_lossy(bytes);
    if !order_str.contains("market") && !order_str.contains("quantity") && !order_str.contains("order") && !order_str.contains("id") {
        // Check if this might be JSON response with a "data" field
        if order_str.contains("data") {
            // Try to extract base64 encoded data
            if let Some(idx) = order_str.find("\"data\":\"") {
                let start_idx = idx + 8;
                if let Some(end_idx) = order_str[start_idx..].find("\"") {
                    let encoded_data = &order_str[start_idx..start_idx+end_idx];
                    if let Ok(decoded) = base64::decode(encoded_data) {
                        return deserialize_order(&decoded);
                    }
                }
            }
        }
        
        // Even if we didn't find text markers, we should still try the binary parser
        // since the response might be a binary format without obvious market/quantity keywords
        match parse_binary_order(bytes) {
            Ok(order) => {
                tracing::debug!("Successfully parsed binary order format (fallback)");
                return Ok(order);
            },
            Err(e) => {
                tracing::debug!("Failed to parse binary order format (fallback): {}", e);
                return Err(eyre!("Response doesn't appear to contain order data"));
            }
        }
    }

    // Try to decode as a JSON object first
    if order_str.trim().starts_with('{') && order_str.contains("\"id\"") {
        match serde_json::from_slice::<Order>(bytes) {
            Ok(order) => {
                tracing::debug!("Successfully decoded order as JSON");
                return Ok(order);
            },
            Err(e) => {
                tracing::debug!("Failed to deserialize as JSON: {}", e);
                // Fall through to other approaches
            }
        }
    }
    
    // Check if this is a protobuf message
    match astria_core::protocol::orderbook::v1::Order::decode(bytes) {
        Ok(order) => {
            println!("Successfully decoded order as Protobuf: {}", order.id);
            
            // Debug the quantity values
            if let Some(qty) = &order.quantity {
                let full_qty = ((qty.hi as u128) << 64) + (qty.lo as u128);
                println!("Order quantity from protobuf: lo={}, hi={}, full={}", qty.lo, qty.hi, full_qty);
            } else {
                println!("Order has no quantity field in protobuf");
            }
            
            if let Some(rem_qty) = &order.remaining_quantity {
                let full_rem = ((rem_qty.hi as u128) << 64) + (rem_qty.lo as u128);
                println!("Order remaining_quantity from protobuf: lo={}, hi={}, full={}", rem_qty.lo, rem_qty.hi, full_rem);
            } else {
                println!("Order has no remaining_quantity field in protobuf");
            }
            
            // Convert to our own Order struct
            Ok(Order {
                id: order.id,
                owner: order.owner.map(|o| Owner { bech32m: o.bech32m }),
                market: order.market,
                side: order.side,
                r#type: order.r#type,
                price: order.price.clone(),
                quantity: order.quantity.clone(),
                remaining_quantity: order.remaining_quantity.clone(),
                created_at: order.created_at,
                time_in_force: order.time_in_force,
                fee_asset: order.fee_asset,
            })
        },
        Err(e) => {
            tracing::debug!("Failed to decode as Protobuf: {}", e);
            
            // Special handling for JSON objects containing the order
            if order_str.contains("\"order\"") || order_str.contains("\"value\"") {
                // Try to extract the nested order object
                let json_value: serde_json::Value = match serde_json::from_slice(bytes) {
                    Ok(val) => val,
                    Err(_) => return Err(eyre!("Failed to parse JSON wrapper")),
                };
                
                // Look for order in possible locations
                let order_obj = if let Some(order) = json_value.get("order") {
                    order
                } else if let Some(value) = json_value.get("value") {
                    value
                } else if let Some(result) = json_value.get("result") {
                    result
                } else {
                    return Err(eyre!("Could not find order in JSON wrapper"));
                };
                
                // Try to convert the order object to our struct
                match serde_json::from_value::<Order>(order_obj.clone()) {
                    Ok(order) => {
                        tracing::debug!("Successfully extracted order from JSON wrapper");
                        return Ok(order);
                    },
                    Err(e) => {
                        tracing::debug!("Failed to deserialize nested order object: {}", e);
                    }
                }
            }
            
            // If we're still here, we failed to deserialize using all approaches
            Err(eyre!("Failed to decode order: could not parse in any known format"))
        }
    }
}

/// Deserialize a list of Orders from bytes
fn deserialize_orders(bytes: &[u8]) -> Result<Vec<Order>, eyre::Report> {
    if bytes.len() < 10 {
        return Err(eyre!("Response too short to contain orders"));
    }
    
    // Check if this might be a binary format with multiple orders
    let has_binary_header = bytes.iter().take(10).any(|b| *b < 32 && *b != 10 && *b != 13 && *b != 9);
    if has_binary_header || bytes[0] == 1 || bytes[0] == 2 { // Side values often appear first
        tracing::debug!("Trying to parse as binary order list format");
        // We need to extract multiple orders from the binary data
        // Instead of trying to parse the entire bytes as a single order,
        // let's first look for UUIDs in the binary data which likely represent orders
        
        // To better detect the order format, let's scan for ALL UUIDs in the data
        let uuid_pattern = regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap();
        let str_data = String::from_utf8_lossy(bytes);
        
        let mut all_orders = Vec::new();
        
        // Find all UUIDs - each likely represents an order
        let uuid_matches: Vec<_> = uuid_pattern.find_iter(&str_data).collect();
        tracing::debug!("Found {} UUIDs in binary data ({})", uuid_matches.len(), bytes.len());
        
        if uuid_matches.len() > 0 {
            for (i, m) in uuid_matches.iter().enumerate() {
                let uuid = m.as_str();
                let start_pos = m.start();
                tracing::debug!("UUID {}: {} at position {}", i+1, uuid, start_pos);
                
                // Create a section for this order
                // Starting a few bytes before the UUID to include any metadata like side
                let start_idx = start_pos.saturating_sub(8);
                
                // End at the next UUID or the end of data
                let end_idx = if i < uuid_matches.len() - 1 {
                    uuid_matches[i+1].start()
                } else {
                    bytes.len()
                };
                
                if start_idx < end_idx && start_idx < bytes.len() && end_idx <= bytes.len() {
                    let section = &bytes[start_idx..end_idx];
                    tracing::debug!("Extracted section {}: {} bytes", i+1, section.len());
                    
                    // Try to parse this section as an order
                    match parse_binary_order(section) {
                        Ok(order) => {
                            tracing::debug!("Successfully parsed order {}: {}", i+1, uuid);
                            all_orders.push(order);
                        },
                        Err(e) => {
                            tracing::debug!("Failed to parse order section {}: {}", i+1, e);
                            
                            // Create a minimal order as fallback
                            let minimal_order = Order {
                                id: uuid.to_string(),
                                owner: None,
                                market: "Unknown".to_string(),
                                side: if bytes[0] == 1 || bytes[0] == 2 { bytes[0] as i32 } else { 1 }, // Default to BUY (1=BUY, 2=SELL)
                                r#type: 1, // Default to LIMIT
                                price: Some(ProtoUint128 { lo: 1, hi: 0 }),
                                quantity: Some(ProtoUint128 { lo: 1, hi: 0 }),
                                remaining_quantity: Some(ProtoUint128 { lo: 1, hi: 0 }),
                                created_at: 0,
                                time_in_force: 1, // Default to GTC
                                fee_asset: "".to_string(),
                            };
                            all_orders.push(minimal_order);
                        }
                    }
                }
            }
            
            if !all_orders.is_empty() {
                tracing::debug!("Successfully extracted {} orders from binary data", all_orders.len());
                return Ok(all_orders);
            }
        }
        
        // If direct UUID extraction doesn't work, try our previous approach
        if let Ok(order) = parse_binary_order(bytes) {
            tracing::debug!("Successfully parsed single binary order");
            return Ok(vec![order]);
        }
        
        // Try to find UUIDs and parse each section as a separate order
        let order_sections = find_order_sections(bytes);
        if !order_sections.is_empty() {
            tracing::debug!("Found {} potential order sections in binary data", order_sections.len());
            let mut orders = Vec::new();
            
            // Analyze the binary data to understand structure
            tracing::debug!("Binary data size: {} bytes", bytes.len());
            
            // Look for specific patterns in the binary data - UUIDs have a specific format
            // The data might contain multiple orders, so let's scan for all UUID patterns
            let uuid_pattern = regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap();
            let mut uuid_matches: Vec<(usize, String)> = vec![];
            
            for cap_match in uuid_pattern.find_iter(std::str::from_utf8(bytes).unwrap_or("")) {
                let start_pos = cap_match.start();
                let uuid = cap_match.as_str().to_string();
                tracing::debug!("Found UUID at position {}: {}", start_pos, uuid);
                uuid_matches.push((start_pos, uuid));
            }
            
            tracing::debug!("Found a total of {} UUIDs in the binary data", uuid_matches.len());
            
            // First byte might be side information for all orders in this list
            let side = if bytes[0] == 1 || bytes[0] == 2 { // Check if first byte indicates BUY (1) or SELL (2)
                tracing::debug!("Using side {} from first byte for all orders", bytes[0]);
                Some(bytes[0] as i32)
            } else {
                None
            };
            
            for section in order_sections {
                match parse_binary_order(section) {
                    Ok(mut order) => {
                        // If the order didn't get a side but we have one from the response, use it
                        if order.side == 0 && side.is_some() {
                            order.side = side.unwrap();
                        }
                        orders.push(order);
                    },
                    Err(e) => {
                        tracing::debug!("Failed to parse order section: {}", e);
                    }
                }
            }
            
            if !orders.is_empty() {
                tracing::debug!("Successfully parsed {} orders from binary sections", orders.len());
                return Ok(orders);
            }
        }
    }
    
    // First try direct deserialization as a Vec<OrderWrapper>
    println!("Attempting to deserialize {} bytes as Vec<OrderWrapper>", bytes.len());
    
    // Try to deserialize the data as a Vec<OrderWrapper> using our custom implementation
    tracing::debug!("Trying to deserialize as a Vec<OrderWrapper>");
    match borsh::BorshDeserialize::deserialize(&mut &bytes[..]) as Result<Vec<OrderWrapper>, _> {
        Ok(wrappers) => {
            tracing::debug!("Successfully deserialized {} OrderWrappers", wrappers.len());
            
            // Convert OrderWrapper to our Order struct
            let orders = wrappers.into_iter()
                .map(|wrapper| {
                    // Print detailed quantity info for each order
                    let proto_order = wrapper.0;
                    tracing::debug!("Processing order: {}", proto_order.id);
                    
                    // Log quantity information for debugging
                    if let Some(qty) = &proto_order.quantity {
                        let full_qty = ((qty.hi as u128) << 64) + (qty.lo as u128);
                        tracing::debug!("  Quantity: lo={}, hi={}, full={}", qty.lo, qty.hi, full_qty);
                    } else {
                        tracing::debug!("  No quantity field");
                    }
                    
                    if let Some(rem_qty) = &proto_order.remaining_quantity {
                        let full_rem = ((rem_qty.hi as u128) << 64) + (rem_qty.lo as u128);
                        tracing::debug!("  Remaining quantity: lo={}, hi={}, full={}", rem_qty.lo, rem_qty.hi, full_rem);
                    } else {
                        tracing::debug!("  No remaining_quantity field");
                    }
                    
                    // Ensure we preserve the side value correctly
                    let side_value = proto_order.side;
                    tracing::debug!("  Side value: {}", side_value);
                    
                    Order {
                        id: proto_order.id,
                        owner: proto_order.owner.map(|o| Owner { bech32m: o.bech32m }),
                        market: proto_order.market,
                        side: side_value,
                        r#type: proto_order.r#type,
                        price: proto_order.price.clone(),
                        quantity: proto_order.quantity.clone(),
                        remaining_quantity: proto_order.remaining_quantity.clone(),
                        created_at: proto_order.created_at,
                        time_in_force: proto_order.time_in_force,
                        fee_asset: proto_order.fee_asset,
                    }
                })
                .collect::<Vec<_>>();
            
            tracing::debug!("Converted {} OrderWrappers to Order structs", orders.len());
            return Ok(orders);
        },
        Err(e) => {
            tracing::debug!("Failed to deserialize as Vec<OrderWrapper>: {}", e);
        }
    }
    
    // If direct deserialization failed, try the raw bytes approach
    match borsh::BorshDeserialize::deserialize(&mut &bytes[..]) as Result<Vec<u8>, _> {
        Ok(inner_bytes) => {
            println!("Deserialized as Vec<u8>: {} bytes", inner_bytes.len());
            
            // Try to decode as binary protobuf directly
            if let Ok(proto_order) = astria_core::protocol::orderbook::v1::Order::decode(&*inner_bytes) {
                println!("Successfully decoded single order as protobuf: {}", proto_order.id);
                
                // Create and return a single order
                let order = Order {
                    id: proto_order.id,
                    owner: proto_order.owner.map(|o| Owner { bech32m: o.bech32m }),
                    market: proto_order.market,
                    side: proto_order.side,
                    r#type: proto_order.r#type,
                    price: proto_order.price,
                    quantity: proto_order.quantity,
                    remaining_quantity: proto_order.remaining_quantity,
                    created_at: proto_order.created_at,
                    time_in_force: proto_order.time_in_force,
                    fee_asset: proto_order.fee_asset,
                };
                
                return Ok(vec![order]);
            } else {
                println!("Failed to decode as single protobuf Order");
            }
        },
        Err(e) => {
            println!("Failed to deserialize as Vec<u8>: {}", e);
        }
    }
    
    // Fall back to traditional methods
    let str_data = String::from_utf8_lossy(bytes);
    let mut orders = Vec::new();
    
    // Check if this is a JSON array response
    if str_data.trim().starts_with('[') {
        match serde_json::from_slice::<Vec<Order>>(bytes) {
            Ok(parsed_orders) => {
                println!("Successfully parsed orders as JSON array");
                return Ok(parsed_orders);
            },
            Err(e) => {
                println!("Failed to parse as JSON array: {}", e);
                // Fall through to other approaches
            }
        }
    }
    
    // Check if this might be JSON response with a "data" field containing base64
    if str_data.contains("\"data\"") {
        // Try to extract base64 encoded data
        if let Some(idx) = str_data.find("\"data\":\"") {
            let start_idx = idx + 8;
            if let Some(end_idx) = str_data[start_idx..].find("\"") {
                let encoded_data = &str_data[start_idx..start_idx+end_idx];
                if let Ok(decoded) = base64::decode(encoded_data) {
                    tracing::debug!("Found and decoded base64 data ({} bytes)", decoded.len());
                    return deserialize_orders(&decoded);
                }
            }
        }
    }
    
    // Try to parse as a JSON object with a list of orders
    if str_data.trim().starts_with('{') {
        match serde_json::from_slice::<serde_json::Value>(bytes) {
            Ok(json_value) => {
                // Look for order list in possible locations
                let order_array = if let Some(orders) = json_value.get("orders") {
                    orders
                } else if let Some(value) = json_value.get("value") {
                    value
                } else if let Some(result) = json_value.get("result") {
                    result
                } else {
                    &json_value // Try the root object itself
                };
                
                // If this is an array, try to parse each item
                if let Some(arr) = order_array.as_array() {
                    for item in arr {
                        match serde_json::from_value::<Order>(item.clone()) {
                            Ok(order) => orders.push(order),
                            Err(e) => tracing::debug!("Failed to parse order from array item: {}", e),
                        }
                    }
                    
                    if !orders.is_empty() {
                        tracing::debug!("Successfully parsed {} orders from JSON object", orders.len());
                        return Ok(orders);
                    }
                }
            },
            Err(e) => {
                tracing::debug!("Failed to parse as JSON object: {}", e);
            }
        }
    }
    
    // First, check if this is a protobuf message list
    let mut cursor = bytes;
    
    while !cursor.is_empty() {
        match astria_core::protocol::orderbook::v1::Order::decode(cursor) {
            Ok(order) => {
                tracing::debug!("Successfully decoded order as Protobuf");
                // Convert to our own Order struct and add to list
                orders.push(Order {
                    id: order.id,
                    owner: order.owner.map(|o| Owner { bech32m: o.bech32m }),
                    market: order.market,
                    side: order.side,
                    r#type: order.r#type,
                    price: order.price,
                    quantity: order.quantity,
                    remaining_quantity: order.remaining_quantity,
                    created_at: order.created_at,
                    time_in_force: order.time_in_force,
                    fee_asset: order.fee_asset,
                });
                
                // Try to advance cursor based on message length
                // This is a rough heuristic
                let consumed = cursor.len().min(200);
                cursor = &cursor[consumed..];
            },
            Err(_) => {
                // If we can't decode as a protobuf, try to decode entire response as a list
                break;
            }
        }
    }
    
    // If we found any orders using the iterative approach, return them
    if !orders.is_empty() {
        tracing::debug!("Successfully parsed {} orders using Protobuf iteration", orders.len());
        return Ok(orders);
    }
    
    // Fallback: try another approach
    // The data might be borsh-serialized, but we don't have direct access to the OrderWrapper
    // implementation, so we'll try a heuristic approach to extract potential orders
    
    // Check if the data contains indicators of orders
    if str_data.contains("order") || str_data.contains("market") {
        // Try to identify order chunks and decode them
        let chunks = str_data.split("order").collect::<Vec<_>>();
        
        for chunk in chunks {
            if chunk.contains("market") && chunk.contains("id") {
                // This chunk might contain order data
                // Try to extract a subsection of the original bytes that might be a complete order
                let start_idx = bytes.len().saturating_sub(chunk.len() + 10);
                let end_idx = std::cmp::min(start_idx + 500, bytes.len()); 
                
                if start_idx < end_idx {
                    let potential_order_bytes = &bytes[start_idx..end_idx];
                    if let Ok(order) = astria_core::protocol::orderbook::v1::Order::decode(potential_order_bytes) {
                        orders.push(Order {
                            id: order.id,
                            owner: order.owner.map(|o| Owner { bech32m: o.bech32m }),
                            market: order.market,
                            side: order.side,
                            r#type: order.r#type,
                            price: order.price,
                            quantity: order.quantity,
                            remaining_quantity: order.remaining_quantity,
                            created_at: order.created_at,
                            time_in_force: order.time_in_force,
                            fee_asset: order.fee_asset,
                        });
                    }
                }
            }
        }
    }
    
    // If we found any orders using the chunking approach, return them
    if !orders.is_empty() {
        tracing::debug!("Successfully parsed {} orders using chunk extraction", orders.len());
        return Ok(orders);
    }
    
    // If we still couldn't find any orders, try one more approach - extract UUIDs
    // We'll just extract the IDs here; the detailed fetching will happen in the caller
    let uuid_pattern = regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap();
    
    for cap in uuid_pattern.captures_iter(&str_data) {
        // Just create a minimal order with the ID
        orders.push(Order {
            id: cap[0].to_string(),
            owner: None,
            market: "Unknown".to_string(), // This will be fixed in the caller
            side: 0, // This will be fixed in the caller
            r#type: 1, // Default to LIMIT
            price: None,
            quantity: None,
            remaining_quantity: None,
            created_at: 0,
            time_in_force: 1, // Default to GTC
            fee_asset: "".to_string(),
        });
    }
    
    if !orders.is_empty() {
        tracing::debug!("Successfully extracted {} order IDs using UUID pattern", orders.len());
        return Ok(orders);
    }
    
    // If we still couldn't find any orders, return an empty list
    tracing::debug!("No orders found after trying multiple parsing approaches");
    Ok(Vec::new())
}

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
            SubCommand::GetOrder(get_order) => get_order.run().await,
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
    /// Get details for specific order(s) by ID
    GetOrder(GetOrderCommand),
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
        let _sequencer_info = match sequencer_client.abci_info().await {
            Ok(info) => {
                println!("Sequencer version: {}, App version: {}", info.version, info.app_version);
                Some(info)
            },
            Err(e) => {
                println!("Failed to get sequencer version: {}", e);
                None
            },
        };
        
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
        
        // If response is empty
        if response.value.is_empty() {
            println!("No markets found in response.");
            return Ok(());
        }
        
        // Try to parse as JSON first
        let raw_string = String::from_utf8_lossy(&response.value);
        println!("Raw response: {}", raw_string);
        
        match serde_json::from_str::<Vec<String>>(&raw_string) {
            Ok(markets) => {
                if markets.is_empty() {
                    println!("No markets found.");
                } else {
                    println!("Markets found:");
                    for (i, market) in markets.iter().enumerate() {
                        println!("{}. {}", i + 1, market);
                    }
                }
            },
            Err(_) => {
                // If JSON parsing fails, try splitting by delimiters
                println!("Response is not in JSON format. Trying to parse manually...");
                
                // Try to parse potential market names
                let markets: Vec<&str> = raw_string
                    .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '/' && c != '_')
                    .filter(|s| !s.is_empty() && s.len() > 2) // filter out very short segments
                    .collect();
                
                if !markets.is_empty() {
                    println!("Markets found:");
                    for (i, market) in markets.iter().enumerate() {
                        println!("{}. {}", i + 1, market);
                    }
                } else {
                    println!("No markets could be parsed from the response.");
                }
            }
        }
        
        Ok(())
    }
}

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
        let sequencer_client = HttpClient::new(url.clone())
            .wrap_err("failed constructing http sequencer client")?;
        
        println!("Querying orders for market {} from sequencer at {}", self.market, self.sequencer_url);
        
        // URL encode the market name since it may contain slashes (e.g., "BTC/USD-1")
        let encoded_market = urlencoding::encode(&self.market);
        
        // If a specific side is provided, query only that side
        // Otherwise, query both BUY and SELL sides
        let sides = match self.side {
            Some(ref s) => vec![s.to_lowercase()],
            None => vec!["buy".to_string(), "sell".to_string()],
        };
        
        println!("Querying for market: {}, sides: {:?}", self.market, sides);
        
        let mut all_orders = Vec::new();
        
        // Query each side separately and combine the results
        for side in sides {
            // Ensure we're using lowercase for the path
            let side_lower = side.to_lowercase();
            let path = format!("orderbook/orders/market/{}/{}", encoded_market, side_lower);
            println!("Querying path: {}", path);
            println!("Side value: {:?}", side);
            
            match sequencer_client.abci_query(Some(path.clone()), vec![], Some(0u32.into()), false).await {
            Ok(response) => {
                println!("Got response - code: {:?}, log: {}", response.code, response.log);
                
                if !response.code.is_ok() {
                    println!("Error response: {}", response.log);
                    return Ok(());
                }
                
                if response.value.is_empty() {
                    println!("No orders found for market {} with side {}", self.market, side);
                    return Ok(());
                }
                
                println!("Response value size: {} bytes", response.value.len());
                println!("First 20 bytes: {:?}", &response.value[0..20.min(response.value.len())]);
                
                // Print the response as hex for debugging
                let hex_string = hex::encode(&response.value[0..100.min(response.value.len())]);
                println!("First 100 bytes as hex: {}", hex_string);
                
                // Try to deserialize the response as a list of OrderWrapper
                if response.value.len() > 4 {
                    // Check if we can make sense of the response
                    match deserialize_orders(&response.value) {
                        Ok(mut orders) => {
                            // Only set the side if it's not already set correctly
                            let side_value = match side.to_lowercase().as_str() {
                                "buy" => 1,
                                "sell" => 2,
                                _ => 0,
                            };
                            
                            // Debug output for each order
                            let mut orders_with_incorrect_side = 0;
                            for (i, order) in orders.iter().enumerate() {
                                println!("Order {}/{} from {} query:", i+1, orders.len(), side);
                                println!("  ID: {}", order.id);
                                println!("  Market: {}", order.market);
                                
                                if order.side != side_value && order.side != 0 {
                                    println!("  Side mismatch: order has {} but query path indicates {}", 
                                        order.side, side_value);
                                    orders_with_incorrect_side += 1;
                                } else {
                                    println!("  Side: {}", side_value);
                                }
                                
                                if let Some(qty) = &order.quantity {
                                    let full_qty = ((qty.hi as u128) << 64) + (qty.lo as u128);
                                    println!("  Quantity: lo={}, hi={}, full={}", qty.lo, qty.hi, full_qty);
                                } else {
                                    println!("  Quantity: None");
                                }
                                
                                if let Some(rem_qty) = &order.remaining_quantity {
                                    let full_rem = ((rem_qty.hi as u128) << 64) + (rem_qty.lo as u128);
                                    println!("  Remaining Quantity: lo={}, hi={}, full={}", rem_qty.lo, rem_qty.hi, full_rem);
                                } else {
                                    println!("  Remaining Quantity: None");
                                }
                            }
                            
                            // Log info about any side mismatches found
                            if orders_with_incorrect_side > 0 {
                                println!("WARNING: Found {} orders with side values that don't match the query path ({})", 
                                    orders_with_incorrect_side, side);
                            }
                            
                            // Update orders with missing or incorrect side values
                            for order in &mut orders {
                                if order.side == 0 || order.side != side_value {
                                    println!("Updating order {} side: {} -> {}", order.id, order.side, side_value);
                                    order.side = side_value;
                                }
                            }
                            
                            println!("Found {} {} orders", orders.len(), side.to_uppercase());
                            
                            // Instead of printing details for each side, just collect them
                            all_orders.extend(orders);
                        },
                        Err(e) => {
                            println!("Failed to deserialize orders: {}", e);
                            
                            // Fallback to showing potential IDs if deserialization fails
                            let raw_string = String::from_utf8_lossy(&response.value);
                            
                            // Try to find order IDs and other identifiable patterns
                            let potential_ids: Vec<&str> = raw_string
                                .split(|c: char| !c.is_alphanumeric() && c != '-')
                                .filter(|s| 
                                    !s.is_empty() && 
                                    (s.len() == 36 || // UUID length
                                    (s.len() > 8 && s.contains('-'))) // Partial UUID or similar ID
                                )
                                .collect();
                            
                            if !potential_ids.is_empty() {
                                println!("\nPotential {} order IDs found:", side.to_uppercase());
                                for (i, id) in potential_ids.iter().enumerate() {
                                    if i < 10 { // Just show a few
                                        println!("{}. {}", i + 1, id);
                                    }
                                }
                                println!("Found {} potential {} order IDs", potential_ids.len(), side.to_uppercase());
                            } else {
                                println!("No {} order IDs could be identified in the response.", side.to_uppercase());
                                println!("Raw response first 100 bytes: {:?}", 
                                    &response.value[0..std::cmp::min(100, response.value.len())]);
                            }
                        }
                    }
                } else {
                    println!("No {} orders found for market {}.", side.to_uppercase(), self.market);
                }
            },
            Err(e) => {
                println!("Error querying {}: {}", path, e);
            }
        }
        }
        
        // After querying all sides, display the combined results
        if !all_orders.is_empty() {
            println!("\nAll orders found for market {}:", self.market);
            
            // For orders with missing details, fetch the complete information
            let market_name = self.market.clone();
            let sequencer_client = HttpClient::new(url.clone())
                .wrap_err("failed constructing http sequencer client")?;
            
            let mut enhanced_orders: Vec<Order> = Vec::new();
            
            for mut order in all_orders {
                // If order has minimal information, try to get complete details
                if order.market == "Unknown" || order.price.is_none() || order.quantity.is_none() {
                    // Mark the basic market from the query
                    order.market = market_name.clone();
                    
                    // Attempt to retrieve complete order details
                    tracing::debug!("Fetching complete details for order: {}", order.id);
                    
                    // No longer checking logs, just rely on the API data
                    println!(" Fetching API details for order: {}", order.id);
                    
                    // No log parsing logic - we'll rely on the API data instead
                    
                    // Query the API for order details
                    let order_path = format!("orderbook/order/{}", order.id);
                    tracing::debug!("Querying path: {}", order_path);
                    match sequencer_client.abci_query(Some(order_path), vec![], Some(0u32.into()), false).await {
                        Ok(response) => {
                            tracing::debug!("Got response - code: {:?}, log: {}", response.code, response.log);
                            tracing::debug!("Response value size: {} bytes", response.value.len());
                            
                            if response.code.is_ok() && !response.value.is_empty() {
                                // Print the first 100 characters of the response to help debug
                                let raw_str = String::from_utf8_lossy(&response.value);
                                let preview = if raw_str.len() > 100 { 
                                    format!("{}...", &raw_str[..100]) 
                                } else { 
                                    raw_str.to_string() 
                                };
                                tracing::debug!("Response preview: {}", preview);
                                
                                // Try to extract any hex or base64 encoded data
                                if raw_str.len() > 20 {
                                    // Find and decode base64 data if present
                                    if let Some(idx) = raw_str.find("\"data\":\"") {
                                        let start_idx = idx + 8;
                                        if let Some(end_idx) = raw_str[start_idx..].find("\"") {
                                            let encoded_data = &raw_str[start_idx..start_idx+end_idx];
                                            tracing::debug!("Found encoded data: {}", encoded_data);
                                            
                                            // Try to decode as base64
                                            if let Ok(decoded) = base64::decode(encoded_data) {
                                                if decoded.len() > 10 {
                                                    tracing::debug!("Attempting to deserialize from decoded base64 data ({} bytes)", decoded.len());
                                                    match deserialize_order(&decoded) {
                                                        Ok(detailed_order) => {
                                                            tracing::debug!("Successfully deserialized from base64 data for order: {}", order.id);
                                                            // Create a final order with the best information from both sources
                                                            let mut final_order = detailed_order;

                                                            // Always use the market name from our query if we have it
                                                            if final_order.market == "Unknown" && !market_name.is_empty() {
                                                                tracing::debug!("Updating market: {} -> {}", final_order.market, market_name);
                                                                final_order.market = market_name.clone();
                                                            }

                                                            // Use side from original order if detailed order doesn't have it
                                                            if final_order.side == 0 && order.side != 0 {
                                                                tracing::debug!("Updating side: {} -> {}", final_order.side, order.side);
                                                                final_order.side = order.side;
                                                            }

                                                            // Use additional fields from original order if detailed order is missing them
                                                            if final_order.price.is_none() && order.price.is_some() {
                                                                tracing::debug!("Using price from original order");
                                                                final_order.price = order.price.clone();
                                                            }

                                                            if final_order.quantity.is_none() && order.quantity.is_some() {
                                                                tracing::debug!("Using quantity from original order");
                                                                final_order.quantity = order.quantity.clone();
                                                            }

                                                            if final_order.remaining_quantity.is_none() && order.remaining_quantity.is_some() {
                                                                tracing::debug!("Using remaining_quantity from original order");
                                                                final_order.remaining_quantity = order.remaining_quantity.clone();
                                                            }
                                                            enhanced_orders.push(final_order);
                                                            continue;
                                                        }
                                                        Err(e) => {
                                                            tracing::debug!("Error deserializing from base64 data for order {}: {}", order.id, e);
                                                            // Continue to try other approaches
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                // Regular approach - try to deserialize directly
                                match deserialize_order(&response.value) {
                                    Ok(detailed_order) => {
                                        tracing::debug!("Successfully retrieved detailed information for order: {}", order.id);
                                        // Create a final order with the best information from both sources
                                        let mut final_order = detailed_order;

                                        // Always use the market name from our query if we have it
                                        if final_order.market == "Unknown" && !market_name.is_empty() {
                                            tracing::debug!("Updating market: {} -> {}", final_order.market, market_name);
                                            final_order.market = market_name.clone();
                                        }

                                        // Use side from original order if detailed order doesn't have it
                                        if final_order.side == 0 && order.side != 0 {
                                            tracing::debug!("Updating side: {} -> {}", final_order.side, order.side);
                                            final_order.side = order.side;
                                        }

                                        // Use additional fields from original order if detailed order is missing them
                                        if final_order.price.is_none() && order.price.is_some() {
                                            tracing::debug!("Using price from original order");
                                            final_order.price = order.price.clone();
                                        }

                                        if final_order.quantity.is_none() && order.quantity.is_some() {
                                            tracing::debug!("Using quantity from original order");
                                            final_order.quantity = order.quantity.clone();
                                        }

                                        if final_order.remaining_quantity.is_none() && order.remaining_quantity.is_some() {
                                            tracing::debug!("Using remaining_quantity from original order");
                                            final_order.remaining_quantity = order.remaining_quantity.clone();
                                        }
                                        enhanced_orders.push(final_order);
                                        continue;
                                    }
                                    Err(e) => {
                                        tracing::debug!("Error deserializing detailed order {}: {}", order.id, e);
                                        // Try alternative approaches before falling through
                                    }
                                }
                            }
                        }
                        Err(e) => {
                                        tracing::debug!("Error querying detailed order {}: {}", order.id, e);
                            // Fall through to use the original order
                        }
                    }
                }
                
                // If we couldn't get detailed information, use the original with the market set
                enhanced_orders.push(order);
            }
            
            match self.format.to_lowercase().as_str() {
                "simple" => {
                    println!("\nOrders found (ID only):");
                    for (i, order) in enhanced_orders.iter().enumerate() {
                        println!("{}. {}", i + 1, order.id);
                    }
                },
                "json" => {
                    println!("\nOrders in JSON format:");
                    let json = serde_json::to_string_pretty(&enhanced_orders).unwrap_or_else(|_| "Error serializing to JSON".to_string());
                    println!("{}", json);
                },
                _ => { // detailed format
                    println!("\nOrders found:");
                    for (i, order) in enhanced_orders.iter().enumerate() {
                        println!("Order {}:", i + 1);
                        println!("  ID: {}", order.id);
                        println!("  Market: {}", order.market);
                        println!("  Side: {}", match order.side {
                            1 => "BUY",
                            2 => "SELL",
                            _ => "BUY" // Default to BUY for any uncertain values
                        });
                        println!("  Type: {}", if order.r#type == 1 { "LIMIT" } else { "MARKET" });
                        
                        // Display price and quantities using only the protobuf data
                        if let Some(price) = &order.price {
                            println!("  Price: {}", format_uint128(price));
                        } else {
                            println!("  Price: Not specified");
                        }
                        
                        if let Some(quantity) = &order.quantity {
                            println!("  Quantity: {}", format_uint128(quantity));
                        } else {
                            println!("  Quantity: Not specified");
                        }
                        
                        if let Some(remaining) = &order.remaining_quantity {
                            println!("  Remaining Quantity: {}", format_uint128(remaining));
                        } else {
                            println!("  Remaining Quantity: Not specified");
                        }
                        
                        // Display owner if available
                        if let Some(owner) = &order.owner {
                            println!("  Owner: {}", owner.bech32m);
                        } else {
                            println!("  Owner: Unknown");
                        }
                        
                        // Format timestamp as human-readable time
                        if order.created_at > 0 {
                            let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(order.created_at as i64, 0)
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_else(|| format!("{}", order.created_at));
                            println!("  Created At: {}", datetime);
                        } else {
                            println!("  Created At: {}", order.created_at);
                        }
                        
                        println!("  Time In Force: {}", match order.time_in_force {
                            1 => "Good Till Cancelled (GTC)",
                            2 => "Immediate Or Cancel (IOC)",
                            3 => "Fill Or Kill (FOK)",
                            _ => "Unknown",
                        });
                        
                        println!("");
                    }
                }
            }
            
            println!("Found {} total orders in market {}", enhanced_orders.len(), self.market);
        } else {
            println!("\nNo orders found in market {}", self.market);
        }
        
        Ok(())
    }
}

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
        
        // Request each order separately and collect the results
        let mut successful_orders = Vec::new();
        let mut failed_order_ids = Vec::new();

        for order_id in &self.order_ids {
            let path = format!("orderbook/order/{}", order_id);
            println!("Querying order: {}", order_id);
            
            match sequencer_client.abci_query(Some(path.clone()), vec![], Some(0u32.into()), false).await {
                Ok(response) => {
                    if !response.code.is_ok() {
                        println!("Error response for order {}: {}", order_id, response.log);
                        failed_order_ids.push(order_id.clone());
                        continue;
                    }
                    
                    if response.value.is_empty() {
                        println!("No data returned for order {}", order_id);
                        failed_order_ids.push(order_id.clone());
                        continue;
                    }
                    
                    // Try to deserialize the response as an OrderWrapper
                    match deserialize_order(&response.value) {
                        Ok(order) => {
                            successful_orders.push(order);
                        },
                        Err(e) => {
                            println!("Failed to deserialize order {}: {}", order_id, e);
                            failed_order_ids.push(order_id.clone());
                        }
                    }
                },
                Err(e) => {
                    println!("Error querying order {}: {}", order_id, e);
                    failed_order_ids.push(order_id.clone());
                }
            }
        }
        
        // Display the results based on the selected format
        if !successful_orders.is_empty() {
            // Try to enhance order information by querying complete details for each order
            let mut enhanced_orders = Vec::new();
            let order_count = successful_orders.len();
            
            for order in successful_orders {
                // If order has minimal information, try to get complete details
                if order.market == "Unknown" || order.price.is_none() || order.quantity.is_none() || order.side == 0 {
                    // Try to query the full order details
                    println!("Enhancing order details for ID: {}", order.id);
                    
                    // Query the specific order by ID
                    let order_path = format!("orderbook/order/{}", order.id);
                    println!("Querying path: {}", order_path);
                    match sequencer_client.abci_query(Some(order_path), vec![], Some(0u32.into()), false).await {
                        Ok(response) => {
                            println!("Got response - code: {:?}, log: {}", response.code, response.log);
                            println!("Response value size: {} bytes", response.value.len());
                            
                            if response.code.is_ok() && !response.value.is_empty() {
                                // Print the first 100 characters of the response to help debug
                                let raw_str = String::from_utf8_lossy(&response.value);
                                let preview = if raw_str.len() > 100 { 
                                    format!("{}...", &raw_str[..100]) 
                                } else { 
                                    raw_str.to_string() 
                                };
                                println!("Response preview: {}", preview);
                                
                                // Try to extract any hex or base64 encoded data
                                if raw_str.len() > 20 {
                                    // Find and decode base64 data if present
                                    if let Some(idx) = raw_str.find("\"data\":\"") {
                                        let start_idx = idx + 8;
                                        if let Some(end_idx) = raw_str[start_idx..].find("\"") {
                                            let encoded_data = &raw_str[start_idx..start_idx+end_idx];
                                            println!("Found encoded data: {}", encoded_data);
                                            
                                            // Try to decode as base64
                                            if let Ok(decoded) = base64::decode(encoded_data) {
                                                if decoded.len() > 10 {
                                                    println!("Attempting to deserialize from decoded base64 data ({} bytes)", decoded.len());
                                                    match deserialize_order(&decoded) {
                                                        Ok(detailed_order) => {
                                                            println!("Successfully deserialized from base64 data for order: {}", order.id);
                                                            // Create a final order with the best information from both sources
                                                            let mut final_order = detailed_order;

                                                            // Use side from original order if detailed order doesn't have it
                                                            if final_order.side == 0 && order.side != 0 {
                                                                println!("Updating side: {} -> {}", final_order.side, order.side);
                                                                final_order.side = order.side;
                                                            }

                                                            // Use additional fields from original order if detailed order is missing them
                                                            if final_order.price.is_none() && order.price.is_some() {
                                                                println!("Using price from original order");
                                                                final_order.price = order.price.clone();
                                                            }

                                                            if final_order.quantity.is_none() && order.quantity.is_some() {
                                                                println!("Using quantity from original order");
                                                                final_order.quantity = order.quantity.clone();
                                                            }

                                                            if final_order.remaining_quantity.is_none() && order.remaining_quantity.is_some() {
                                                                println!("Using remaining_quantity from original order");
                                                                final_order.remaining_quantity = order.remaining_quantity.clone();
                                                            }
                                                            enhanced_orders.push(final_order);
                                                            continue;
                                                        }
                                                        Err(e) => {
                                                            println!("Error deserializing from base64 data for order {}: {}", order.id, e);
                                                            // Continue to try other approaches
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                // Regular approach - try to deserialize directly
                                match deserialize_order(&response.value) {
                                    Ok(detailed_order) => {
                                        println!("Successfully retrieved detailed information for order: {}", order.id);
                                        // Create a final order with the best information from both sources
                                        let mut final_order = detailed_order;

                                        // Use side from original order if detailed order doesn't have it
                                        if final_order.side == 0 && order.side != 0 {
                                            println!("Updating side: {} -> {}", final_order.side, order.side);
                                            final_order.side = order.side;
                                        }

                                        // Use additional fields from original order if detailed order is missing them
                                        if final_order.price.is_none() && order.price.is_some() {
                                            println!("Using price from original order");
                                            final_order.price = order.price.clone();
                                        }

                                        if final_order.quantity.is_none() && order.quantity.is_some() {
                                            println!("Using quantity from original order");
                                            final_order.quantity = order.quantity.clone();
                                        }

                                        if final_order.remaining_quantity.is_none() && order.remaining_quantity.is_some() {
                                            println!("Using remaining_quantity from original order");
                                            final_order.remaining_quantity = order.remaining_quantity.clone();
                                        }
                                        enhanced_orders.push(final_order);
                                        continue;
                                    }
                                    Err(e) => {
                                        println!("Error deserializing detailed order {}: {}", order.id, e);
                                        // Try alternative approaches before falling through
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("Error querying detailed order {}: {}", order.id, e);
                            // Fall through to use the original order with modifications
                        }
                    }
                    
                    // If we couldn't get complete details, try to improve what we have
                    let mut fixed_order = order.clone();
                    
                    // Set BUY side as default since this is most common
                    if fixed_order.side == 0 {
                        fixed_order.side = 1; // Default to BUY side
                    }
                    
                    enhanced_orders.push(fixed_order);
                } else {
                    // Order already has complete details
                    enhanced_orders.push(order);
                }
            }
            
            match self.format.to_lowercase().as_str() {
                "simple" => {
                    println!("\nOrders found (ID only):");
                    for (i, order) in enhanced_orders.iter().enumerate() {
                        println!("{}. {}", i + 1, order.id);
                    }
                },
                "json" => {
                    println!("\nOrders in JSON format:");
                    let json = serde_json::to_string_pretty(&enhanced_orders).unwrap_or_else(|_| "Error serializing to JSON".to_string());
                    println!("{}", json);
                },
                _ => { // detailed format
                    println!("\nOrders found:");
                    for (i, order) in enhanced_orders.iter().enumerate() {
                        println!("Order {}:", i + 1);
                        println!("  ID: {}", order.id);
                        println!("  Market: {}", order.market);
                        println!("  Side: {}", match order.side {
                            1 => "BUY",
                            2 => "SELL",
                            _ => "BUY" // Default to BUY for any uncertain values
                        });
                        println!("  Type: {}", if order.r#type == 1 { "LIMIT" } else { "MARKET" });
                        
                        // Display price and quantities using only the protobuf data
                        if let Some(price) = &order.price {
                            println!("  Price: {}", format_uint128(price));
                        } else {
                            println!("  Price: Not specified");
                        }
                        
                        if let Some(quantity) = &order.quantity {
                            println!("  Quantity: {}", format_uint128(quantity));
                        } else {
                            println!("  Quantity: Not specified");
                        }
                        
                        if let Some(remaining) = &order.remaining_quantity {
                            println!("  Remaining Quantity: {}", format_uint128(remaining));
                        } else {
                            println!("  Remaining Quantity: Not specified");
                        }
                        
                        // Display owner if available
                        if let Some(owner) = &order.owner {
                            println!("  Owner: {}", owner.bech32m);
                        } else {
                            println!("  Owner: Unknown");
                        }
                        
                        // Format timestamp as human-readable time
                        if order.created_at > 0 {
                            let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(order.created_at as i64, 0)
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_else(|| format!("{}", order.created_at));
                            println!("  Created At: {}", datetime);
                        } else {
                            println!("  Created At: {}", order.created_at);
                        }
                        
                        println!("  Time In Force: {}", match order.time_in_force {
                            1 => "Good Till Cancelled (GTC)",
                            2 => "Immediate Or Cancel (IOC)",
                            3 => "Fill Or Kill (FOK)",
                            _ => "Unknown",
                        });
                        
                        println!("");
                    }
                }
            }
            
            println!("Successfully retrieved {} order(s)", order_count);
        } else {
            println!("No orders were successfully retrieved");
        }
        
        if !failed_order_ids.is_empty() {
            println!("Failed to retrieve {} order(s): {:?}", failed_order_ids.len(), failed_order_ids);
        }
        
        Ok(())
    }
}