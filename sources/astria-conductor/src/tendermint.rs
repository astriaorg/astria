use color_eyre::eyre::{self, WrapErr as _};
use reqwest::{Client, Response as ReqwestResponse};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;

static VALIDATOR_SET_ENDPOINT: &str = "/cosmos/base/tendermint/v1beta1/validatorsets/";

#[derive(Serialize, Debug)]
pub struct EmptyRequest {}

#[derive(Deserialize, Debug)]
pub struct ValidatorSet {
    pub block_height: String,
    pub validators: Vec<Validator>,
}

impl ValidatorSet {
    /// returns the proposer given the current set by ordering the validators by proposer priority.
    /// the validator with the highest proposer priority is the proposer.
    /// TODO: could there ever be two validators with the same priority?
    pub(crate) fn get_proposer(&mut self) -> eyre::Result<Validator> {
        self.validators.sort_by(|v1, v2| {
            v1.proposer_priority
                .parse::<i64>()
                .unwrap()
                .cmp(&v2.proposer_priority.parse::<i64>().unwrap())
        });
        self.validators
            .first()
            .cloned()
            .ok_or_else(|| eyre::eyre!("no proposer found"))
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Validator {
    pub address: String,
    pub pub_key: KeyWithType,
    pub voting_power: String,
    pub proposer_priority: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct KeyWithType {
    #[serde(rename = "@type")]
    pub key_type: String,
    pub key: String,
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

    pub async fn get_proposer_address(&self, height: u64) -> eyre::Result<String> {
        let mut validator_set = self.get_validator_set(height).await?;
        let proposer = validator_set.get_proposer()?;
        Ok(proposer.address)
    }

    pub async fn get_validator_set(&self, height: u64) -> eyre::Result<ValidatorSet> {
        let endpoint: String = format!("{}{}{}", self.endpoint, VALIDATOR_SET_ENDPOINT, height);
        self.do_get::<EmptyRequest, ValidatorSet>(endpoint, None)
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

#[cfg(test)]
mod test {
    use super::TendermintClient;

    #[tokio::test]
    async fn test_get_validator_set() {
        let cosmos_endpoint = "http://localhost:1317".to_string();
        let client = TendermintClient::new(cosmos_endpoint).unwrap();
        let resp = client.get_validator_set(1).await.unwrap();
        println!("ValidatorSet: {:?}", resp);
    }
}
