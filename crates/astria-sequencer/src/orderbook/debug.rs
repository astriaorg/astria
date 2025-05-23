use cnidarium::{StateRead, StateWrite};
use futures::StreamExt;
use tracing::{debug, info, warn};

use crate::orderbook::state_ext::{MarketParams, StateReadExt};
use crate::storage::{keys, StoredValue};

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