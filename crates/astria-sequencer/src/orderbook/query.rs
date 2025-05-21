use std::str::FromStr;

use astria_core::{
    primitive::v1::{Address, Bech32m},
    protocol::{
        abci::AbciErrorCode,
        orderbook::v1::{Order, OrderSide}
    }
};
use borsh::BorshSerialize;
use crate::orderbook::compat::{OrderWrapper, OrderMatchWrapper, OrderbookWrapper};
use cnidarium::{
    StateRead,
    Storage,
};
use futures::StreamExt;
use tendermint::abci::{
    request,
    response,
    Code,
};

use crate::orderbook::state_ext::StateReadExt;

/// Handles orderbook ABCI query request.
pub(crate) async fn orderbook_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    let snapshot = storage.latest_snapshot();

    // Get market from params
    let market = match params.iter().find_map(|(k, v)| (k == "market").then_some(v)) {
        Some(market) => market,
        None => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
                log: "Path parameter 'market' not found".to_string(),
                info: AbciErrorCode::INVALID_PARAMETER.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };
    
    // Check if market exists
    if !snapshot.market_exists(market) {
        return response::Query {
            code: Code::Err(AbciErrorCode::VALUE_NOT_FOUND.value()),
            log: format!("Market not found: {}", market),
            info: "The requested market does not exist".to_string(),
            index: 0,
            key: request.data.clone(),
            value: Vec::new().into(),
            proof: None,
            height: tendermint::block::Height::from(0_u32),
            codespace: "".to_string(),
        };
    }

    // Get the orderbook
    let orderbook = snapshot.get_orderbook(market);

    // Serialize the response
    let wrapped_orderbook = OrderbookWrapper(orderbook);
    let value = match borsh::to_vec(&wrapped_orderbook) {
        Ok(bytes) => bytes,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                log: format!("Failed to serialize orderbook: {}", err),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };

    response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: request.data.clone(),
        value: value.into(),
        proof: None,
        height: tendermint::block::Height::from(0_u32),
        codespace: "".to_string(),
    }
}

pub(crate) async fn order_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    let snapshot = storage.latest_snapshot();

    // Get order_id from params
    let order_id = match params.iter().find_map(|(k, v)| (k == "order_id").then_some(v)) {
        Some(order_id) => order_id,
        None => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
                log: "Path parameter 'order_id' not found".to_string(),
                info: AbciErrorCode::INVALID_PARAMETER.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };
    
    // Get the order
    let order = match snapshot.get_order(order_id) {
        Some(order) => order,
        None => {
            return response::Query {
                code: Code::Err(AbciErrorCode::VALUE_NOT_FOUND.value()),
                log: format!("Order not found: {}", order_id),
                info: "The requested order does not exist".to_string(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };

    // Serialize the response
    let value = match borsh::to_vec(&OrderWrapper(order)) {
        Ok(bytes) => bytes,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                log: format!("Failed to serialize order: {}", err),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };

    response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: request.data.clone(),
        value: value.into(),
        proof: None,
        height: tendermint::block::Height::from(0_u32),
        codespace: "".to_string(),
    }
}

pub(crate) async fn market_orders_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    let snapshot = storage.latest_snapshot();

    // Get market from params
    let market = match params.iter().find_map(|(k, v)| (k == "market").then_some(v)) {
        Some(market) => market,
        None => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
                log: "Path parameter 'market' not found".to_string(),
                info: AbciErrorCode::INVALID_PARAMETER.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };
    
    // Check if market exists
    if !snapshot.market_exists(market) {
        return response::Query {
            code: Code::Err(AbciErrorCode::VALUE_NOT_FOUND.value()),
            log: format!("Market not found: {}", market),
            info: "The requested market does not exist".to_string(),
            index: 0,
            key: request.data.clone(),
            value: Vec::new().into(),
            proof: None,
            height: tendermint::block::Height::from(0_u32),
            codespace: "".to_string(),
        };
    }

    // Get side from params (optional)
    let side = params.iter().find_map(|(k, v)| {
        if k == "side" {
            match v.as_str() {
                "buy" => Some(OrderSide::Buy),
                "sell" => Some(OrderSide::Sell),
                _ => None,
            }
        } else {
            None
        }
    });

    // Get orders
    let orders: Vec<Order> = snapshot.get_market_orders(market, side);

    // Serialize the response
    let wrapped_orders: Vec<OrderWrapper> = orders.into_iter().map(OrderWrapper).collect();
    let value = match borsh::to_vec(&wrapped_orders) {
        Ok(bytes) => bytes,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                log: format!("Failed to serialize orders: {}", err),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };

    response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: request.data.clone(),
        value: value.into(),
        proof: None,
        height: tendermint::block::Height::from(0_u32),
        codespace: "".to_string(),
    }
}

pub(crate) async fn owner_orders_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    let snapshot = storage.latest_snapshot();

    // Get owner from params
    let owner = match params.iter().find_map(|(k, v)| (k == "owner").then_some(v)) {
        Some(owner) => owner,
        None => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
                log: "Path parameter 'owner' not found".to_string(),
                info: AbciErrorCode::INVALID_PARAMETER.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };
    
    // Validate owner address
    match Address::<Bech32m>::from_str(owner) {
        Ok(_) => {},
        Err(_) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
                log: format!("Invalid owner address: {}", owner),
                info: "The provided address is not a valid Astria address".to_string(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };

    // Get orders
    let orders: Vec<Order> = snapshot.get_owner_orders(owner);

    // Serialize the response
    let wrapped_orders: Vec<OrderWrapper> = orders.into_iter().map(OrderWrapper).collect();
    let value = match borsh::to_vec(&wrapped_orders) {
        Ok(bytes) => bytes,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                log: format!("Failed to serialize orders: {}", err),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };

    response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: request.data.clone(),
        value: value.into(),
        proof: None,
        height: tendermint::block::Height::from(0_u32),
        codespace: "".to_string(),
    }
}

pub(crate) async fn markets_request(
    storage: Storage,
    request: request::Query,
    _params: Vec<(String, String)>,
) -> response::Query {
    // Add very prominent logging for this handler
    tracing::warn!(" GET MARKETS QUERY HANDLER CALLED ");
    
    let snapshot = storage.latest_snapshot();

    // No params required for this endpoint
    
    // Check markets storage and add debug logs
    let all_markets_key = crate::storage::keys::orderbook_all_markets();
    let markets_prefix = crate::storage::keys::orderbook_markets();
    
    tracing::warn!(" Checking ALL_MARKETS at key: {}", all_markets_key);
    match futures::executor::block_on(snapshot.get_raw(all_markets_key.as_str())) {
        Ok(Some(bytes)) => {
            tracing::warn!(" Found ALL_MARKETS data ({} bytes)", bytes.len());
            
            match crate::storage::StoredValue::deserialize(&bytes) {
                Ok(crate::storage::StoredValue::Bytes(inner_bytes)) => {
                    match borsh::from_slice::<Vec<String>>(&inner_bytes) {
                        Ok(markets_list) => {
                            tracing::warn!(" Successfully deserialized markets list: {:?}", markets_list);
                        },
                        Err(e) => {
                            tracing::warn!(" Failed to deserialize inner bytes into markets list: {:?}", e);
                        }
                    }
                },
                Ok(other) => {
                    tracing::warn!(" StoredValue isn't Bytes but: {:?}", other);
                },
                Err(e) => {
                    tracing::warn!(" Failed to deserialize ALL_MARKETS as StoredValue: {:?}", e);
                }
            }
        },
        Ok(None) => {
            tracing::warn!(" ALL_MARKETS key not found");
        },
        Err(e) => {
            tracing::warn!(" Error reading ALL_MARKETS key: {:?}", e);
        }
    }
    
    tracing::warn!(" Checking markets prefix: {}", markets_prefix);
    
    // Check keys under the markets prefix
    futures::executor::block_on(async {
        let stream = snapshot.prefix_raw(&markets_prefix);
        futures::pin_mut!(stream);
        
        let mut count = 0;
        while let Some(result) = stream.next().await {
            match result {
                Ok((key_bytes, value_bytes)) => {
                    let key = String::from_utf8_lossy(key_bytes.as_bytes());
                    let value = String::from_utf8_lossy(value_bytes.as_slice());
                    tracing::warn!(" Found market at key {}: {:?}", key, value);
                    count += 1;
                },
                Err(e) => {
                    tracing::warn!(" Error during prefix scan: {:?}", e);
                }
            }
        }
        
        if count == 0 {
            tracing::warn!(" No markets found under markets prefix");
        } else {
            tracing::warn!(" Found {} markets under markets prefix", count);
        }
    });
    
    // Debug all keys starting with "orderbook/"
    futures::executor::block_on(async {
        tracing::warn!(" Scanning all orderbook/ keys");
        let stream = snapshot.prefix_raw("orderbook/");
        futures::pin_mut!(stream);
        
        let mut count = 0;
        while let Some(result) = stream.next().await {
            match result {
                Ok((key_bytes, value_bytes)) => {
                    let key = String::from_utf8_lossy(key_bytes.as_bytes());
                    let value = String::from_utf8_lossy(value_bytes.as_slice());
                    tracing::warn!("- Key: {}, Value: {:?}", key, value);
                    count += 1;
                },
                Err(e) => {
                    tracing::warn!("Error scanning orderbook keys: {:?}", e);
                }
            }
        }
        
        if count == 0 {
            tracing::warn!(" No keys found under orderbook/ prefix");
        } else {
            tracing::warn!(" Found {} keys under orderbook/ prefix", count);
        }
    });
    
    // Get markets (after all the debug)
    let markets: Vec<String> = snapshot.get_markets();
    tracing::warn!(" Markets from get_markets(): {:?}", markets);

    // Simple JSON serialization for better compatibility
    let markets_json = match serde_json::to_string(&markets) {
        Ok(json) => {
            tracing::warn!(" Successfully serialized markets to JSON: {}", json);
            json.into_bytes()
        },
        Err(err) => {
            tracing::warn!(" Failed to serialize markets to JSON, falling back to plain text format: {}", err);
            // Fall back to simple concatenation with newlines
            markets.join("\n").into_bytes()
        }
    };

    tracing::warn!(" Returning markets response with {} markets", markets.len());
    response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: request.data.clone(),
        value: markets_json.into(),
        proof: None,
        height: tendermint::block::Height::from(0_u32),
        codespace: "".to_string(),
    }
}

pub(crate) async fn market_params_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    let snapshot = storage.latest_snapshot();

    // Get market from params
    let market = match params.iter().find_map(|(k, v)| (k == "market").then_some(v)) {
        Some(market) => market,
        None => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
                log: "Path parameter 'market' not found".to_string(),
                info: AbciErrorCode::INVALID_PARAMETER.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };
    
    // Get market parameters
    let params = match snapshot.get_market_params(market) {
        Some(params) => params,
        None => {
            return response::Query {
                code: Code::Err(AbciErrorCode::VALUE_NOT_FOUND.value()),
                log: format!("Market not found: {}", market),
                info: "The requested market does not exist".to_string(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };

    // Serialize the response
    let value = match borsh::to_vec(&params) {
        Ok(bytes) => bytes,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                log: format!("Failed to serialize market parameters: {}", err),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };

    response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: request.data.clone(),
        value: value.into(),
        proof: None,
        height: tendermint::block::Height::from(0_u32),
        codespace: "".to_string(),
    }
}

pub(crate) async fn trades_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    let snapshot = storage.latest_snapshot();

    // Get market from params
    let market = match params.iter().find_map(|(k, v)| (k == "market").then_some(v)) {
        Some(market) => market,
        None => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
                log: "Path parameter 'market' not found".to_string(),
                info: AbciErrorCode::INVALID_PARAMETER.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };
    
    // Check if market exists
    if !snapshot.market_exists(market) {
        return response::Query {
            code: Code::Err(AbciErrorCode::VALUE_NOT_FOUND.value()),
            log: format!("Market not found: {}", market),
            info: "The requested market does not exist".to_string(),
            index: 0,
            key: request.data.clone(),
            value: Vec::new().into(),
            proof: None,
            height: tendermint::block::Height::from(0_u32),
            codespace: "".to_string(),
        };
    }

    // Get optional limit parameter
    let limit = match params.iter().find_map(|(k, v)| (k == "limit").then_some(v)) {
        Some(limit_str) => match limit_str.parse::<usize>() {
            Ok(l) => l,
            Err(_) => {
                return response::Query {
                    code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
                    log: format!("Invalid limit: {}", limit_str),
                    info: "Limit must be a positive integer".to_string(),
                    index: 0,
                    key: request.data.clone(),
                    value: Vec::new().into(),
                    proof: None,
                    height: tendermint::block::Height::from(0_u32),
                    codespace: "".to_string(),
                };
            }
        },
        None => 10, // Default limit
    };

    // Get trades
    let trades = snapshot.get_recent_trades(market, limit);

    // Serialize the response
    let wrapped_trades: Vec<OrderMatchWrapper> = trades.into_iter().map(OrderMatchWrapper).collect();
    let value = match borsh::to_vec(&wrapped_trades) {
        Ok(bytes) => bytes,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                log: format!("Failed to serialize trades: {}", err),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                index: 0,
                key: request.data.clone(),
                value: Vec::new().into(),
                proof: None,
                height: tendermint::block::Height::from(0_u32),
                codespace: "".to_string(),
            };
        }
    };

    response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: request.data.clone(),
        value: value.into(),
        proof: None,
        height: tendermint::block::Height::from(0_u32),
        codespace: "".to_string(),
    }
}

