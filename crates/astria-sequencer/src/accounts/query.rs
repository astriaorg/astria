use std::collections::VecDeque;

use anyhow::{
    anyhow,
    bail,
    Context,
    Result,
};
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use tracing::instrument;

use crate::accounts::{
    state_ext::StateReadExt,
    types::{
        Address,
        Balance,
        Nonce,
    },
};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub(crate) enum QueryRequest {
    BalanceQuery(Address),
    NonceQuery(Address),
}

impl QueryRequest {
    pub(crate) fn decode(mut path: VecDeque<&str>) -> Result<QueryRequest> {
        let query_type = path.pop_front().ok_or(anyhow!("missing query type"))?;
        let address = path.pop_front().ok_or(anyhow!("missing address"))?;

        match query_type {
            "balance" => Ok(QueryRequest::BalanceQuery(
                Address::try_from(address).context("failed to parse address")?,
            )),
            "nonce" => Ok(QueryRequest::NonceQuery(
                Address::try_from(address).context("failed to parse address")?,
            )),
            other => bail!("invalid query type: `{other}`"),
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum QueryResponse {
    BalanceResponse(Balance),
    NonceResponse(Nonce),
}

#[derive(Default)]
pub(crate) struct QueryHandler {}

impl QueryHandler {
    pub(crate) fn new() -> Self {
        Self {}
    }

    #[instrument(skip(self, state))]
    pub(crate) async fn handle<S: StateReadExt>(
        &self,
        state: S,
        query: QueryRequest,
    ) -> Result<QueryResponse> {
        match query {
            QueryRequest::BalanceQuery(address) => {
                let balance = state
                    .get_account_balance(&address)
                    .await
                    .context("failed getting account balance")?;
                Ok(QueryResponse::BalanceResponse(balance))
            }
            QueryRequest::NonceQuery(address) => {
                let nonce = state
                    .get_account_nonce(&address)
                    .await
                    .context("failed getting account nonce")?;
                Ok(QueryResponse::NonceResponse(nonce))
            }
        }
    }
}
