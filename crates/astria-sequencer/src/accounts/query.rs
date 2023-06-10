use std::collections::VecDeque;

use anyhow::{
    anyhow,
    bail,
    Context as _,
    Result,
};
use borsh::BorshSerialize as _;
use tracing::instrument;

use crate::accounts::{
    state_ext::StateReadExt,
    types::{
        Address,
        Balance,
        Nonce,
    },
};

#[derive(Debug)]
pub enum QueryRequest {
    BalanceQuery(Address),
    NonceQuery(Address),
}

impl QueryRequest {
    pub fn decode(mut path: VecDeque<&str>) -> Result<QueryRequest> {
        let query_type = path.pop_front().ok_or(anyhow!("missing query type"))?;
        let address = path.pop_front().ok_or(anyhow!("missing address"))?;

        match query_type {
            "balance" => Ok(QueryRequest::BalanceQuery(Address::from(address))),
            "nonce" => Ok(QueryRequest::NonceQuery(Address::from(address))),
            _ => bail!("invalid query type"),
        }
    }
}

pub enum QueryResponse {
    BalanceResponse(Balance),
    NonceResponse(Nonce),
}

impl QueryResponse {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        match self {
            QueryResponse::BalanceResponse(balance) => Ok(balance
                .try_to_vec()
                .context("failed to serialize balance")?),
            QueryResponse::NonceResponse(nonce) => {
                Ok(nonce.try_to_vec().context("failed to serialize nonce")?)
            }
        }
    }
}

#[derive(Default)]
pub struct QueryHandler {}

impl QueryHandler {
    pub fn new() -> Self {
        Self {}
    }

    #[instrument(skip(self, state))]
    pub async fn handle<S: StateReadExt>(
        &self,
        state: S,
        query: QueryRequest,
    ) -> Result<QueryResponse> {
        match query {
            QueryRequest::BalanceQuery(address) => {
                let balance = state.get_account_balance(&address).await?;
                Ok(QueryResponse::BalanceResponse(balance))
            }
            QueryRequest::NonceQuery(address) => {
                let nonce = state.get_account_nonce(&address).await?;
                Ok(QueryResponse::NonceResponse(nonce))
            }
        }
    }
}
