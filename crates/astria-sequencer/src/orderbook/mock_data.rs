use cnidarium::{StateRead, StateWrite};
use tracing::info;

use crate::orderbook::state_ext::{MarketParams, StateWriteExt};

/// Insert a test market into the database for testing purposes
pub fn insert_test_market<S: StateWrite>(state: &mut S) -> Result<(), String> {
    // Create market parameters
    let market_params = MarketParams {
        base_asset: "BTC".to_string(),
        quote_asset: "USD".to_string(),
        tick_size: Some(100), // $1.00
        lot_size: Some(1000000), // 0.01 BTC
        paused: false,
    };
    
    // Add the market to state
    match state.add_market("BTC/USD", market_params) {
        Ok(_) => {
            info!(
                market = "BTC/USD",
                base_asset = "BTC",
                quote_asset = "USD",
                "Added test market successfully"
            );
            Ok(())
        },
        Err(e) => {
            info!(
                market = "BTC/USD",
                error = ?e,
                "Failed to add test market"
            );
            Err(format!("Failed to add test market: {:?}", e))
        }
    }
}

/// Check if markets exist
pub fn check_markets<S: StateRead>(state: &S) -> Vec<String> {
    use crate::orderbook::state_ext::StateReadExt;
    state.get_markets()
}