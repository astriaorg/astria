use std::str::FromStr;

use astria_core::{
    primitive::v1::Address,
    protocol::{
        abci::AbciErrorCode,
        orderbook::v1::{Order, OrderSide, OrderMatch, Orderbook}
    }
};
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use borsh::BorshSerialize;
use cnidarium::{
    Snapshot,
    StateRead,
    Storage,
};
use serde::{Deserialize, Serialize};
use tendermint::abci::{
    request,
    response,
    Code,
};

use crate::orderbook::state_ext::{MarketParams, StateReadExt};

/// Handles orderbook ABCI query request.
pub async fn orderbook_request(
    storage: Storage,
    req: &request::Query,
) -> Result<response::Query> {
    let snapshot = storage.latest_snapshot();

    // Parse market from the path
    let parts: Vec<&str> = req.path.split('/').collect();
    if parts.len() < 3 || parts[0] != "orderbook" {
        return Ok(response::Query {
            code: Code::Err(AbciErrorCode::InvalidRequest as u32),
            log: format!("Invalid path: {}", req.path),
            info: "Expected path format: orderbook/:market".to_string(),
            index: 0,
            key: req.data.clone(),
            value: Vec::new(),
            proof: None,
            height: 0,
            codespace: "".to_string(),
        });
    }

    let market = parts[2];
    
    // Check if market exists
    if !snapshot.market_exists(market) {
        return Ok(response::Query {
            code: Code::Err(AbciErrorCode::NotFound as u32),
            log: format!("Market not found: {}", market),
            info: "The requested market does not exist".to_string(),
            index: 0,
            key: req.data.clone(),
            value: Vec::new(),
            proof: None,
            height: 0,
            codespace: "".to_string(),
        });
    }

    // Get the orderbook
    let orderbook = snapshot.get_orderbook(market);

    // Serialize the response
    let value = match orderbook.try_to_vec() {
        Ok(bytes) => bytes,
        Err(err) => {
            return Ok(response::Query {
                code: Code::Err(AbciErrorCode::InternalError as u32),
                log: format!("Failed to serialize orderbook: {}", err),
                info: "Internal error".to_string(),
                index: 0,
                key: req.data.clone(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            });
        }
    };

    Ok(response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: req.data.clone(),
        value,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

pub async fn order_request(
    storage: Storage,
    req: &request::Query,
) -> Result<response::Query> {
    let snapshot = storage.latest_snapshot();

    // Parse order_id from the path
    let parts: Vec<&str> = req.path.split('/').collect();
    if parts.len() < 3 || parts[0] != "orderbook" || parts[1] != "order" {
        return Ok(response::Query {
            code: Code::Err(AbciErrorCode::InvalidRequest as u32),
            log: format!("Invalid path: {}", req.path),
            info: "Expected path format: orderbook/order/:order_id".to_string(),
            index: 0,
            key: req.data.clone(),
            value: Vec::new(),
            proof: None,
            height: 0,
            codespace: "".to_string(),
        });
    }

    let order_id = parts[2];
    
    // Get the order
    let order = match snapshot.get_order(order_id) {
        Some(order) => order,
        None => {
            return Ok(response::Query {
                code: Code::Err(AbciErrorCode::NotFound as u32),
                log: format!("Order not found: {}", order_id),
                info: "The requested order does not exist".to_string(),
                index: 0,
                key: req.data.clone(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            });
        }
    };

    // Serialize the response
    let value = match order.try_to_vec() {
        Ok(bytes) => bytes,
        Err(err) => {
            return Ok(response::Query {
                code: Code::Err(AbciErrorCode::InternalError as u32),
                log: format!("Failed to serialize order: {}", err),
                info: "Internal error".to_string(),
                index: 0,
                key: req.data.clone(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            });
        }
    };

    Ok(response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: req.data.clone(),
        value,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

pub async fn market_orders_request(
    storage: Storage,
    req: &request::Query,
) -> Result<response::Query> {
    let snapshot = storage.latest_snapshot();

    // Parse market and optional side from the path
    let parts: Vec<&str> = req.path.split('/').collect();
    if parts.len() < 4 || parts[0] != "orderbook" || parts[1] != "orders" || parts[2] != "market" {
        return Ok(response::Query {
            code: Code::Err(AbciErrorCode::InvalidRequest as u32),
            log: format!("Invalid path: {}", req.path),
            info: "Expected path format: orderbook/orders/market/:market[/:side]".to_string(),
            index: 0,
            key: req.data.clone(),
            value: Vec::new(),
            proof: None,
            height: 0,
            codespace: "".to_string(),
        });
    }

    let market = parts[3];
    
    // Check if market exists
    if !snapshot.market_exists(market) {
        return Ok(response::Query {
            code: Code::Err(AbciErrorCode::NotFound as u32),
            log: format!("Market not found: {}", market),
            info: "The requested market does not exist".to_string(),
            index: 0,
            key: req.data.clone(),
            value: Vec::new(),
            proof: None,
            height: 0,
            codespace: "".to_string(),
        });
    }

    // Parse optional side parameter
    let side = if parts.len() > 4 {
        match parts[4] {
            "buy" => Some(OrderSide::ORDER_SIDE_BUY),
            "sell" => Some(OrderSide::ORDER_SIDE_SELL),
            _ => None,
        }
    } else {
        None
    };

    // Get orders
    let orders: Vec<Order> = snapshot.get_market_orders(market, side).collect();

    // Serialize the response
    let value = match orders.try_to_vec() {
        Ok(bytes) => bytes,
        Err(err) => {
            return Ok(response::Query {
                code: Code::Err(AbciErrorCode::InternalError as u32),
                log: format!("Failed to serialize orders: {}", err),
                info: "Internal error".to_string(),
                index: 0,
                key: req.data.clone(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            });
        }
    };

    Ok(response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: req.data.clone(),
        value,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

pub async fn owner_orders_request(
    storage: Storage,
    req: &request::Query,
) -> Result<response::Query> {
    let snapshot = storage.latest_snapshot();

    // Parse owner from the path
    let parts: Vec<&str> = req.path.split('/').collect();
    if parts.len() < 4 || parts[0] != "orderbook" || parts[1] != "orders" || parts[2] != "owner" {
        return Ok(response::Query {
            code: Code::Err(AbciErrorCode::InvalidRequest as u32),
            log: format!("Invalid path: {}", req.path),
            info: "Expected path format: orderbook/orders/owner/:owner".to_string(),
            index: 0,
            key: req.data.clone(),
            value: Vec::new(),
            proof: None,
            height: 0,
            codespace: "".to_string(),
        });
    }

    let owner = parts[3];
    
    // Validate owner address
    let address = match Address::from_str(owner) {
        Ok(_) => owner,
        Err(_) => {
            return Ok(response::Query {
                code: Code::Err(AbciErrorCode::InvalidRequest as u32),
                log: format!("Invalid owner address: {}", owner),
                info: "The provided address is not a valid Astria address".to_string(),
                index: 0,
                key: req.data.clone(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            });
        }
    };

    // Get orders
    let orders: Vec<Order> = snapshot.get_owner_orders(address).collect();

    // Serialize the response
    let value = match orders.try_to_vec() {
        Ok(bytes) => bytes,
        Err(err) => {
            return Ok(response::Query {
                code: Code::Err(AbciErrorCode::InternalError as u32),
                log: format!("Failed to serialize orders: {}", err),
                info: "Internal error".to_string(),
                index: 0,
                key: req.data.clone(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            });
        }
    };

    Ok(response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: req.data.clone(),
        value,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

pub async fn markets_request(
    storage: Storage,
    req: &request::Query,
) -> Result<response::Query> {
    let snapshot = storage.latest_snapshot();

    // Parse path
    let parts: Vec<&str> = req.path.split('/').collect();
    if parts.len() < 2 || parts[0] != "orderbook" || parts[1] != "markets" {
        return Ok(response::Query {
            code: Code::Err(AbciErrorCode::InvalidRequest as u32),
            log: format!("Invalid path: {}", req.path),
            info: "Expected path format: orderbook/markets".to_string(),
            index: 0,
            key: req.data.clone(),
            value: Vec::new(),
            proof: None,
            height: 0,
            codespace: "".to_string(),
        });
    }

    // Get markets
    let markets: Vec<String> = snapshot.get_markets().collect();

    // Serialize the response
    let value = match markets.try_to_vec() {
        Ok(bytes) => bytes,
        Err(err) => {
            return Ok(response::Query {
                code: Code::Err(AbciErrorCode::InternalError as u32),
                log: format!("Failed to serialize markets: {}", err),
                info: "Internal error".to_string(),
                index: 0,
                key: req.data.clone(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            });
        }
    };

    Ok(response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: req.data.clone(),
        value,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

pub async fn market_params_request(
    storage: Storage,
    req: &request::Query,
) -> Result<response::Query> {
    let snapshot = storage.latest_snapshot();

    // Parse market from the path
    let parts: Vec<&str> = req.path.split('/').collect();
    if parts.len() < 3 || parts[0] != "orderbook" || parts[1] != "market_params" {
        return Ok(response::Query {
            code: Code::Err(AbciErrorCode::InvalidRequest as u32),
            log: format!("Invalid path: {}", req.path),
            info: "Expected path format: orderbook/market_params/:market".to_string(),
            index: 0,
            key: req.data.clone(),
            value: Vec::new(),
            proof: None,
            height: 0,
            codespace: "".to_string(),
        });
    }

    let market = parts[2];
    
    // Get market parameters
    let params = match snapshot.get_market_params(market) {
        Some(params) => params,
        None => {
            return Ok(response::Query {
                code: Code::Err(AbciErrorCode::NotFound as u32),
                log: format!("Market not found: {}", market),
                info: "The requested market does not exist".to_string(),
                index: 0,
                key: req.data.clone(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            });
        }
    };

    // Serialize the response
    let value = match params.try_to_vec() {
        Ok(bytes) => bytes,
        Err(err) => {
            return Ok(response::Query {
                code: Code::Err(AbciErrorCode::InternalError as u32),
                log: format!("Failed to serialize market parameters: {}", err),
                info: "Internal error".to_string(),
                index: 0,
                key: req.data.clone(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            });
        }
    };

    Ok(response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: req.data.clone(),
        value,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

pub async fn trades_request(
    storage: Storage,
    req: &request::Query,
) -> Result<response::Query> {
    let snapshot = storage.latest_snapshot();

    // Parse market and optional limit from the path
    let parts: Vec<&str> = req.path.split('/').collect();
    if parts.len() < 3 || parts[0] != "orderbook" || parts[1] != "trades" {
        return Ok(response::Query {
            code: Code::Err(AbciErrorCode::InvalidRequest as u32),
            log: format!("Invalid path: {}", req.path),
            info: "Expected path format: orderbook/trades/:market[/:limit]".to_string(),
            index: 0,
            key: req.data.clone(),
            value: Vec::new(),
            proof: None,
            height: 0,
            codespace: "".to_string(),
        });
    }

    let market = parts[2];
    
    // Check if market exists
    if !snapshot.market_exists(market) {
        return Ok(response::Query {
            code: Code::Err(AbciErrorCode::NotFound as u32),
            log: format!("Market not found: {}", market),
            info: "The requested market does not exist".to_string(),
            index: 0,
            key: req.data.clone(),
            value: Vec::new(),
            proof: None,
            height: 0,
            codespace: "".to_string(),
        });
    }

    // Parse optional limit parameter
    let limit = if parts.len() > 3 {
        match parts[3].parse::<usize>() {
            Ok(l) => l,
            Err(_) => {
                return Ok(response::Query {
                    code: Code::Err(AbciErrorCode::InvalidRequest as u32),
                    log: format!("Invalid limit: {}", parts[3]),
                    info: "Limit must be a positive integer".to_string(),
                    index: 0,
                    key: req.data.clone(),
                    value: Vec::new(),
                    proof: None,
                    height: 0,
                    codespace: "".to_string(),
                });
            }
        }
    } else {
        10 // Default limit
    };

    // Get trades
    let trades = snapshot.get_recent_trades(market, limit);

    // Serialize the response
    let value = match trades.try_to_vec() {
        Ok(bytes) => bytes,
        Err(err) => {
            return Ok(response::Query {
                code: Code::Err(AbciErrorCode::InternalError as u32),
                log: format!("Failed to serialize trades: {}", err),
                info: "Internal error".to_string(),
                index: 0,
                key: req.data.clone(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            });
        }
    };

    Ok(response::Query {
        code: Code::Ok,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: req.data.clone(),
        value,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

