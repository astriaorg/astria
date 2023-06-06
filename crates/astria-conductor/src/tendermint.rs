use std::time::Duration;

use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use reqwest::{
    Client,
    Response as ReqwestResponse,
};
use serde::{
    de::DeserializeOwned,
    Serialize,
};
use tendermint::{
    account,
    validator,
};
use tendermint_rpc::endpoint::validators;

static VALIDATOR_SET_ENDPOINT: &str = "/cosmos/base/tendermint/v1beta1/validatorsets/";

#[derive(Serialize, Debug)]
pub struct EmptyRequest {}

/// returns the proposer given the current set by ordering the validators by proposer priority.
/// the validator with the highest proposer priority is the proposer.
/// TODO: could there ever be two validators with the same priority?
pub(crate) fn get_first_proposer(
    validator_set: &mut validators::Response,
) -> eyre::Result<validator::Info> {
    validator_set
        .validators
        .sort_by(|v1, v2| v1.proposer_priority.cmp(&v2.proposer_priority));
    validator_set
        .validators
        .first()
        .cloned()
        .ok_or_else(|| eyre::eyre!("no proposer found"))
}

pub struct TendermintClient {
    endpoint: String,
    http_client: Client,
}

impl TendermintClient {
    pub fn new(endpoint: String) -> eyre::Result<Self> {
        let http_client: Client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        Ok(Self {
            endpoint,
            http_client,
        })
    }

    pub async fn get_proposer_address(&self, height: u64) -> eyre::Result<account::Id> {
        let mut validator_set = self.get_validator_set(height).await?;
        let proposer = get_first_proposer(&mut validator_set)?;
        Ok(proposer.address)
    }

    pub async fn get_validator_set(&self, height: u64) -> eyre::Result<validators::Response> {
        let endpoint: String = format!("{}{}{}", self.endpoint, VALIDATOR_SET_ENDPOINT, height);
        self.do_get::<EmptyRequest, validators::Response>(endpoint, None)
            .await
            .wrap_err_with(|| format!("failed to get validator set at height `{height}`"))
    }

    async fn do_get<Req: Serialize + Sized, Resp: DeserializeOwned>(
        &self,
        endpoint: String,
        req: Option<Req>,
    ) -> eyre::Result<Resp> {
        let response: ReqwestResponse = self.http_client.get(&endpoint).json(&req).send().await?;
        response
            .error_for_status()
            .wrap_err("server returned error status")?
            .json::<Resp>()
            .await
            .wrap_err("failed reading server response as json")
    }
}
