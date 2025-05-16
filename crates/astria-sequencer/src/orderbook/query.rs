use std::str::FromStr;

use astria_core::{
    primitive::v1::Address,
    protocol::{
        abci::AbciErrorCode,
        orderbook::v1::{Order, OrderSide, OrderMatch, Orderbook}
    }
};
use borsh::BorshSerialize;
use cnidarium::{read_only_state, StateRead};
use serde::{Deserialize, Serialize};
use tendermint::abci::{request, response};
use thiserror::Error;
use tower_abci::BoxError;

use crate::orderbook::state_ext::{MarketParams, StateReadExt};

/// Errors that can occur during order book queries.
#[derive(Debug, Error)]
pub enum QueryError {
    #[error("Invalid query path: {0}")]
    InvalidPath(String),
    #[error("Invalid query data: {0}")]
    InvalidData(String),
    #[error("Market not found: {0}")]
    MarketNotFound(String),
    #[error("Order not found: {0}")]
    OrderNotFound(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] std::io::Error),
}

// Query handler functions for ABCI router
pub async fn orderbook_request(
    snapshot: read_only_state!(impl StateRead),
    parts: Vec<&str>,
) -> Result<response::Query, BoxError> {
    let market = parts[0];
    match query_orderbook(&snapshot, market) {
        Ok(response) => Ok(response),
        Err(err) => {
            let response = response::Query {
                code: AbciErrorCode::UnknownError as u32,
                log: err.to_string(),
                info: "".to_string(),
                index: 0,
                key: Vec::new(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            };
            Ok(response)
        }
    }
}

pub async fn order_request(
    snapshot: read_only_state!(impl StateRead),
    parts: Vec<&str>,
) -> Result<response::Query, BoxError> {
    let order_id = parts[0];
    match query_order(&snapshot, order_id) {
        Ok(response) => Ok(response),
        Err(err) => {
            let response = response::Query {
                code: AbciErrorCode::UnknownError as u32,
                log: err.to_string(),
                info: "".to_string(),
                index: 0,
                key: Vec::new(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            };
            Ok(response)
        }
    }
}

pub async fn market_orders_request(
    snapshot: read_only_state!(impl StateRead),
    parts: Vec<&str>,
) -> Result<response::Query, BoxError> {
    let market = parts[0];
    let side = if parts.len() > 1 {
        match parts[1] {
            "buy" => Some(OrderSide::ORDER_SIDE_BUY),
            "sell" => Some(OrderSide::ORDER_SIDE_SELL),
            _ => None,
        }
    } else {
        None
    };
    match query_market_orders(&snapshot, market, side) {
        Ok(response) => Ok(response),
        Err(err) => {
            let response = response::Query {
                code: AbciErrorCode::UnknownError as u32,
                log: err.to_string(),
                info: "".to_string(),
                index: 0,
                key: Vec::new(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            };
            Ok(response)
        }
    }
}

pub async fn owner_orders_request(
    snapshot: read_only_state!(impl StateRead),
    parts: Vec<&str>,
) -> Result<response::Query, BoxError> {
    let owner = parts[0];
    match query_owner_orders(&snapshot, owner) {
        Ok(response) => Ok(response),
        Err(err) => {
            let response = response::Query {
                code: AbciErrorCode::UnknownError as u32,
                log: err.to_string(),
                info: "".to_string(),
                index: 0,
                key: Vec::new(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            };
            Ok(response)
        }
    }
}

pub async fn markets_request(
    snapshot: read_only_state!(impl StateRead),
    _parts: Vec<&str>,
) -> Result<response::Query, BoxError> {
    match query_markets(&snapshot) {
        Ok(response) => Ok(response),
        Err(err) => {
            let response = response::Query {
                code: AbciErrorCode::UnknownError as u32,
                log: err.to_string(),
                info: "".to_string(),
                index: 0,
                key: Vec::new(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            };
            Ok(response)
        }
    }
}

pub async fn market_params_request(
    snapshot: read_only_state!(impl StateRead),
    parts: Vec<&str>,
) -> Result<response::Query, BoxError> {
    let market = parts[0];
    match query_market_params(&snapshot, market) {
        Ok(response) => Ok(response),
        Err(err) => {
            let response = response::Query {
                code: AbciErrorCode::UnknownError as u32,
                log: err.to_string(),
                info: "".to_string(),
                index: 0,
                key: Vec::new(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            };
            Ok(response)
        }
    }
}

pub async fn trades_request(
    snapshot: read_only_state!(impl StateRead),
    parts: Vec<&str>,
) -> Result<response::Query, BoxError> {
    let market = parts[0];
    let limit = if parts.len() > 1 {
        parts[1].parse::<usize>().unwrap_or(10)
    } else {
        10
    };
    match query_trades(&snapshot, market, limit) {
        Ok(response) => Ok(response),
        Err(err) => {
            let response = response::Query {
                code: AbciErrorCode::UnknownError as u32,
                log: err.to_string(),
                info: "".to_string(),
                index: 0,
                key: Vec::new(),
                value: Vec::new(),
                proof: None,
                height: 0,
                codespace: "".to_string(),
            };
            Ok(response)
        }
    }
}

/// Process queries related to the order book.
pub fn process_query(
    state: &impl StateRead,
    req: &request::Query,
) -> Result<response::Query, QueryError> {
    let path = req.path.as_str();
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() < 2 || parts[0] != "orderbook" {
        return Err(QueryError::InvalidPath(path.to_string()));
    }

    match parts[1] {
        "order" => {
            if parts.len() < 3 {
                return Err(QueryError::InvalidPath(path.to_string()));
            }
            let order_id = parts[2];
            query_order(state, order_id)
        }
        "orders" => {
            if parts.len() < 3 {
                return Err(QueryError::InvalidPath(path.to_string()));
            }
            let param_type = parts[2];
            match param_type {
                "market" => {
                    if parts.len() < 4 {
                        return Err(QueryError::InvalidPath(path.to_string()));
                    }
                    let market = parts[3];
                    let side = if parts.len() >= 5 {
                        match parts[4] {
                            "buy" => Some(OrderSide::ORDER_SIDE_BUY),
                            "sell" => Some(OrderSide::ORDER_SIDE_SELL),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    query_market_orders(state, market, side)
                }
                "owner" => {
                    if parts.len() < 4 {
                        return Err(QueryError::InvalidPath(path.to_string()));
                    }
                    let owner = parts[3];
                    query_owner_orders(state, owner)
                }
                _ => Err(QueryError::InvalidPath(path.to_string())),
            }
        }
        "orderbook" => {
            if parts.len() < 3 {
                return Err(QueryError::InvalidPath(path.to_string()));
            }
            let market = parts[2];
            query_orderbook(state, market)
        }
        "markets" => query_markets(state),
        "market_params" => {
            if parts.len() < 3 {
                return Err(QueryError::InvalidPath(path.to_string()));
            }
            let market = parts[2];
            query_market_params(state, market)
        }
        "trades" => {
            if parts.len() < 3 {
                return Err(QueryError::InvalidPath(path.to_string()));
            }
            let market = parts[2];
            let limit = if parts.len() >= 4 {
                parts[3].parse::<usize>().unwrap_or(10)
            } else {
                10
            };
            query_trades(state, market, limit)
        }
        _ => Err(QueryError::InvalidPath(path.to_string())),
    }
}

/// Query a specific order by ID.
fn query_order(state: &impl StateRead, order_id: &str) -> Result<response::Query, QueryError> {
    let order = state
        .get_order(order_id)
        .ok_or_else(|| QueryError::OrderNotFound(order_id.to_string()))?;

    let data = order.try_to_vec()?;

    Ok(response::Query {
        code: 0,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: order_id.as_bytes().to_vec(),
        value: data,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

/// Query orders for a specific market, optionally filtered by side.
fn query_market_orders(
    state: &impl StateRead,
    market: &str,
    side: Option<OrderSide>,
) -> Result<response::Query, QueryError> {
    if !state.market_exists(market) {
        return Err(QueryError::MarketNotFound(market.to_string()));
    }

    let orders: Vec<Order> = state.get_market_orders(market, side).collect();
    let data = orders.try_to_vec()?;

    Ok(response::Query {
        code: 0,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: market.as_bytes().to_vec(),
        value: data,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

/// Query orders for a specific owner.
fn query_owner_orders(
    state: &impl StateRead,
    owner: &str,
) -> Result<response::Query, QueryError> {
    let address = match Address::from_str(owner) {
        Ok(addr) => addr.to_string(),
        Err(_) => return Err(QueryError::InvalidData(format!("Invalid address: {}", owner))),
    };

    let orders: Vec<Order> = state.get_owner_orders(&address).collect();
    let data = orders.try_to_vec()?;

    Ok(response::Query {
        code: 0,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: owner.as_bytes().to_vec(),
        value: data,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

/// Query the order book for a specific market.
fn query_orderbook(state: &impl StateRead, market: &str) -> Result<response::Query, QueryError> {
    if !state.market_exists(market) {
        return Err(QueryError::MarketNotFound(market.to_string()));
    }

    let orderbook = state.get_orderbook(market);
    let data = orderbook.try_to_vec()?;

    Ok(response::Query {
        code: 0,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: market.as_bytes().to_vec(),
        value: data,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

/// Query all available markets.
fn query_markets(state: &impl StateRead) -> Result<response::Query, QueryError> {
    let markets: Vec<String> = state.get_markets().collect();
    let data = markets.try_to_vec()?;

    Ok(response::Query {
        code: 0,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: vec![],
        value: data,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

/// Query parameters for a specific market.
fn query_market_params(
    state: &impl StateRead,
    market: &str,
) -> Result<response::Query, QueryError> {
    let params = state
        .get_market_params(market)
        .ok_or_else(|| QueryError::MarketNotFound(market.to_string()))?;

    let data = params.try_to_vec()?;

    Ok(response::Query {
        code: 0,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: market.as_bytes().to_vec(),
        value: data,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}

/// Query recent trades for a specific market.
fn query_trades(
    state: &impl StateRead,
    market: &str,
    limit: usize,
) -> Result<response::Query, QueryError> {
    if !state.market_exists(market) {
        return Err(QueryError::MarketNotFound(market.to_string()));
    }

    let trades = state.get_recent_trades(market, limit);
    let data = trades.try_to_vec()?;

    Ok(response::Query {
        code: 0,
        log: "".to_string(),
        info: "".to_string(),
        index: 0,
        key: market.as_bytes().to_vec(),
        value: data,
        proof: None,
        height: 0,
        codespace: "".to_string(),
    })
}