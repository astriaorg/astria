use cnidarium::{StateRead, StateWrite};
use futures::StreamExt;
use tracing::{debug, info, warn};

use crate::orderbook::state_ext::{MarketParams, StateReadExt, StateWriteExt};
use crate::storage::{keys, StoredValue};

/// Manually insert a test market into the database
pub fn force_insert_test_market<S: StateWrite>(state: &mut S) -> Result<(), String> {
    // Create market parameters
    let market_params = MarketParams {
        base_asset: "BTC".to_string(),
        quote_asset: "USD".to_string(),
        tick_size: Some(100),
        lot_size: Some(1000000),
        paused: false,
    };
    
    let market_id = "BTC/USD";
    
    // Debug key values
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
    let mut markets = vec![market_id.to_string()];
    
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
    match crate::storage::StoredValue::MarketParams(market_params).serialize() {
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
    let market_id = "BTC/USD";
    
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
    
    // Check all markets from high-level interface
    let markets = state.get_markets();
    info!("Markets from get_markets(): {:?}", markets);
    
    // Check if market exists using high-level interface
    let exists = state.market_exists(market_id);
    info!("Market exists using market_exists(): {}", exists);
    
    // Debug check using prefix_raw to directly examine keys with 'orderbook/'
    futures::executor::block_on(async {
        info!("Dumping all orderbook/ keys:");
        let stream = state.prefix_raw("orderbook/");
        futures::pin_mut!(stream);
        
        while let Some(result) = stream.next().await {
            match result {
                Ok((key, value)) => {
                    info!("- Key: {}, Value: {:?}", key, String::from_utf8_lossy(&value));
                }
                Err(e) => {
                    warn!("Error iterating orderbook keys: {:?}", e);
                }
            }
        }
    });
}