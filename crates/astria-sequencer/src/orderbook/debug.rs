use cnidarium::{StateRead, StateWrite};
use futures::StreamExt;
use tracing::{debug, info, warn};

use crate::orderbook::state_ext::{MarketParams, StateReadExt};
use crate::storage::{keys, StoredValue};

/// Manually insert a test market into the database
pub fn force_insert_test_market<S: StateWrite>(state: &mut S) -> Result<(), String> {
    // Create multiple test markets with different asset types
    
    // First market - standard BTC/USD pair (for basic testing)
    let btc_usd_params = MarketParams {
        base_asset: "BTC".to_string(),
        quote_asset: "USD".to_string(),
        tick_size: Some(100),
        lot_size: Some(1000000),
        paused: false,
    };
    
    // Second market - using ntia (the primary token in the system)
    let ntia_usdc_params = MarketParams {
        base_asset: "ntia".to_string(),
        quote_asset: "usdc".to_string(),
        tick_size: Some(100),
        lot_size: Some(1000000),
        paused: false,
    };
    
    // Third market - additional test market with IBC token (as backup)
    // This uses the format used when testing
    let ibc_usdc_params = MarketParams {
        base_asset: "ibc/54aa0250dd7fd58e88d18dc149d826c5c23bef81e53e0598b37ce5323ab36c30".to_string(),
        quote_asset: "usdc".to_string(), 
        tick_size: Some(100),
        lot_size: Some(1000000),
        paused: false,
    };
    
    // Information about markets being created
    tracing::warn!(" Creating test markets for orderbook testing:");
    tracing::warn!("  - BTC/USD: BTC base asset (for general testing)");
    tracing::warn!("  - NTIA/USDC: ntia base asset (primary test market)");
    tracing::warn!("  - IBC/USDC: IBC token base asset (fallback test market)");
    
    // Insert markets directly
    if let Err(err) = insert_test_market_internal(state, "BTC/USD", &btc_usd_params) {
        warn!("Failed to insert BTC/USD market: {}", err);
    }
    
    if let Err(err) = insert_test_market_internal(state, "NTIA/USDC", &ntia_usdc_params) {
        warn!("Failed to insert NTIA/USDC market: {}", err);
    }
    
    if let Err(err) = insert_test_market_internal(state, "IBC/USDC", &ibc_usdc_params) {
        warn!("Failed to insert IBC/USDC market: {}", err);
    }
    
    // Additional check to make sure NTIA/USDC market params were saved
    let ntia_market_id = "NTIA/USDC";
    
    // Verify entry with direct key
    let market_params_key = keys::orderbook_market_params(ntia_market_id);
    tracing::warn!(" Verifying NTIA/USDC market params at key: {}", market_params_key);
    
    // Store the market again directly as extra insurance (redundant but helpful)
    state.put_raw(
        keys::orderbook_market(ntia_market_id),
        ntia_market_id.as_bytes().to_vec(),
    );
    
    // Store market parameters directly as well
    match crate::storage::StoredValue::MarketParams(ntia_usdc_params.clone()).serialize() {
        Ok(serialized) => {
            state.put_raw(
                market_params_key,
                serialized,
            );
            tracing::warn!(" Directly wrote NTIA/USDC market params as additional verification");
        },
        Err(e) => {
            warn!("Failed to serialize NTIA/USDC market parameters: {:?}", e);
        }
    }
    
    info!("Test markets created successfully");
    return Ok(());
}

/// Helper function to insert a single test market
fn insert_test_market_internal<S: StateWrite>(
    state: &mut S, 
    market_id: &str,
    market_params: &MarketParams
) -> Result<(), String> {
    // Debug key values
    info!("🏦 Adding market '{}' with base={}, quote={}", 
          market_id, market_params.base_asset, market_params.quote_asset);
    debug!("Adding market with key: {}", keys::orderbook_market(market_id));
    debug!("Using markets list key: {}", keys::orderbook_markets());
    debug!("Using all markets key: {}", keys::orderbook_all_markets());
    debug!("Using market params key: {}", keys::orderbook_market_params(market_id));
    
    // 1. Store the market directly
    state.put_raw(
        keys::orderbook_market(market_id),
        market_id.as_bytes().to_vec(),
    );
    
    info!("Directly wrote market entry for market key: {}", keys::orderbook_market(market_id));

    // 2. Add to ALL_MARKETS key (this is what was missing!)
    // First get any existing markets list
    let existing_markets = futures::executor::block_on(async {
        let all_markets_key = keys::orderbook_all_markets();
        let all_markets_bytes = state.get_raw(all_markets_key.as_str()).await;
        
        match all_markets_bytes {
            Ok(Some(bytes)) => {
                // Try to deserialize as StoredValue and then as Vec<String>
                if let Ok(StoredValue::Bytes(inner_bytes)) = StoredValue::deserialize(&bytes) {
                    if let Ok(markets) = borsh::from_slice::<Vec<String>>(&inner_bytes) {
                        info!("Found existing markets: {:?}", markets);
                        markets
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            },
            _ => Vec::new(),
        }
    });
    
    // Add our new market if it's not already there
    let mut markets = existing_markets;
    if !markets.contains(&market_id.to_string()) {
        markets.push(market_id.to_string());
    }
    
    // Write back the updated list
    match crate::storage::StoredValue::Bytes(borsh::to_vec(&markets).unwrap_or_default()).serialize() {
        Ok(serialized) => {
            state.put_raw(
                keys::orderbook_all_markets(),
                serialized,
            );
            info!("Directly wrote to ALL_MARKETS at key: {}", keys::orderbook_all_markets());
        },
        Err(e) => {
            warn!("Failed to write ALL_MARKETS: {:?}", e);
        }
    }
    
    // 3. Store market parameters
    match crate::storage::StoredValue::MarketParams(market_params.clone()).serialize() {
        Ok(serialized) => {
            state.put_raw(
                keys::orderbook_market_params(market_id),
                serialized,
            );
            info!("Directly wrote market params at key: {}", keys::orderbook_market_params(market_id));
            Ok(())
        },
        Err(e) => {
            warn!("Failed to serialize market parameters: {:?}", e);
            Err(format!("Failed to serialize market parameters: {:?}", e))
        }
    }
}

/// Verify market data was stored correctly
pub fn debug_check_market_data<S: StateRead>(state: &S) {
    // Check multiple markets
    let test_markets = vec!["BTC/USD", "NTIA/USDC"];
    
    for market_id in test_markets {
        info!("🔍 Checking market: {}", market_id);
        check_market_data(state, market_id);
    }
    
    // Additional global checks
    // Check all markets from high-level interface
    let markets = state.get_markets();
    info!("Markets from get_markets(): {:?}", markets);
    
    // Debug check using prefix_raw to directly examine keys with 'orderbook/'
    futures::executor::block_on(async {
        info!("Dumping all orderbook/ keys:");
        let stream = state.prefix_raw("orderbook/");
        futures::pin_mut!(stream);
        
        while let Some(result) = stream.next().await {
            match result {
                Ok((key_bytes, value_bytes)) => {
                    let key = String::from_utf8_lossy(key_bytes.as_bytes());
                    let value = String::from_utf8_lossy(value_bytes.as_slice());
                    info!("- Key: {}, Value: {:?}", key, value);
                }
                Err(e) => {
                    warn!("Error iterating orderbook keys: {:?}", e);
                }
            }
        }
    });
}

/// Helper to check a single market's data
pub fn check_market_data<S: StateRead>(state: &S, market_id: &str) {
    
    // Check if we can directly get the market
    let raw_market = futures::executor::block_on(state.get_raw(keys::orderbook_market(market_id).as_str()));
    match raw_market {
        Ok(Some(bytes)) => {
            info!("Found market directly at key {}: {:?}", 
                 keys::orderbook_market(market_id),
                 String::from_utf8_lossy(&bytes));
        },
        Ok(None) => {
            warn!("Market not found at key {}", keys::orderbook_market(market_id));
        },
        Err(e) => {
            warn!("Error getting market at key {}: {:?}", keys::orderbook_market(market_id), e);
        }
    }
    
    // Check if we can get the markets list
    let raw_markets_list = futures::executor::block_on(state.get_raw(keys::orderbook_markets().as_str()));
    match raw_markets_list {
        Ok(Some(bytes)) => {
            info!("Found markets list at key {}: {:?}", 
                 keys::orderbook_markets(),
                 String::from_utf8_lossy(&bytes));
        },
        Ok(None) => {
            warn!("Markets list not found at key {}", keys::orderbook_markets());
        },
        Err(e) => {
            warn!("Error getting markets list at key {}: {:?}", keys::orderbook_markets(), e);
        }
    }
    
    // Check if we can get the ALL_MARKETS entry
    let raw_all_markets = futures::executor::block_on(state.get_raw(keys::orderbook_all_markets().as_str()));
    match raw_all_markets {
        Ok(Some(bytes)) => {
            info!("Found ALL_MARKETS at key {}", keys::orderbook_all_markets());
            
            // Try to deserialize it
            match StoredValue::deserialize(&bytes) {
                Ok(StoredValue::Bytes(inner_bytes)) => {
                    // Try to deserialize the inner bytes as a Vec<String>
                    match borsh::from_slice::<Vec<String>>(&inner_bytes) {
                        Ok(markets) => {
                            info!("Deserialized ALL_MARKETS: {:?}", markets);
                        },
                        Err(e) => {
                            warn!("Failed to deserialize ALL_MARKETS inner bytes: {:?}", e);
                        }
                    }
                },
                Ok(_) => {
                    warn!("ALL_MARKETS value is not StoredValue::Bytes");
                },
                Err(e) => {
                    warn!("Failed to deserialize ALL_MARKETS as StoredValue: {:?}", e);
                }
            }
        },
        Ok(None) => {
            warn!("ALL_MARKETS not found at key {}", keys::orderbook_all_markets());
        },
        Err(e) => {
            warn!("Error getting ALL_MARKETS at key {}: {:?}", keys::orderbook_all_markets(), e);
        }
    }
    
    // Check if we can get the market params
    let raw_market_params = futures::executor::block_on(state.get_raw(keys::orderbook_market_params(market_id).as_str()));
    match raw_market_params {
        Ok(Some(bytes)) => {
            info!("Found market params at key {}", keys::orderbook_market_params(market_id));
            
            // Try to deserialize it
            match StoredValue::deserialize(&bytes) {
                Ok(StoredValue::MarketParams(params)) => {
                    info!("Deserialized market params: {:?}", params);
                },
                Ok(_) => {
                    warn!("Market params value is not StoredValue::MarketParams");
                },
                Err(e) => {
                    warn!("Failed to deserialize market params: {:?}", e);
                }
            }
        },
        Ok(None) => {
            warn!("Market params not found at key {}", keys::orderbook_market_params(market_id));
        },
        Err(e) => {
            warn!("Error getting market params at key {}: {:?}", keys::orderbook_market_params(market_id), e);
        }
    }
    
    // Check if market exists using high-level interface
    let exists = state.market_exists(market_id);
    info!("Market exists using market_exists(): {}", exists);
    
    // Try to get market params using high-level interface
    if let Some(params) = state.get_market_params(market_id) {
        info!("📗 Market params via get_market_params() for {}:", market_id);
        info!("  Base asset: {}", params.base_asset);
        info!("  Quote asset: {}", params.quote_asset);
        info!("  Tick size: {:?}", params.tick_size);
        info!("  Lot size: {:?}", params.lot_size);
        info!("  Paused: {}", params.paused);
        
        // Try to parse the base asset as a Denom
        match params.base_asset.parse::<astria_core::primitive::v1::asset::Denom>() {
            Ok(denom) => {
                let asset_prefixed: astria_core::primitive::v1::asset::IbcPrefixed = denom.into();
                info!("✅ Base asset '{}' successfully parsed as denom: {}", params.base_asset, asset_prefixed);
            },
            Err(err) => {
                warn!("❌ Base asset '{}' failed to parse as denom: {}", params.base_asset, err);
            }
        }
    } else {
        warn!("❌ No market params found via get_market_params() for: {}", market_id);
    }
}