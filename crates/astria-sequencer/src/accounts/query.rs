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
pub(crate) enum Request {
    BalanceQuery(Address),
    NonceQuery(Address),
}

impl Request {
    pub(crate) fn decode(mut path: VecDeque<&str>) -> Result<Self> {
        let query_type = path.pop_front().ok_or(anyhow!("missing query type"))?;
        let address = path.pop_front().ok_or(anyhow!("missing address"))?;

        match query_type {
            "balance" => Ok(Self::BalanceQuery(Address::try_from_str(address).context(
                "failed to parse address while constructing balance query request",
            )?)),
            "nonce" => Ok(Self::NonceQuery(Address::try_from_str(address).context(
                "failed to parse address while constructing nonce query request",
            )?)),
            other => bail!("invalid query type: `{other}`"),
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum Response {
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
        query: Request,
    ) -> Result<Response> {
        match query {
            Request::BalanceQuery(address) => {
                let balance = state
                    .get_account_balance(&address)
                    .await
                    .context("failed getting account balance")?;
                Ok(Response::BalanceResponse(balance))
            }
            Request::NonceQuery(address) => {
                let nonce = state
                    .get_account_nonce(&address)
                    .await
                    .context("failed getting account nonce")?;
                Ok(Response::NonceResponse(nonce))
            }
        }
    }
}
