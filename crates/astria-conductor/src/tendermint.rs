use std::{
    fmt::Display,
    str::FromStr,
    time::Duration,
};

use astria_sequencer_relayer::base64_string::Base64String;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use reqwest::{
    Client,
    Response as ReqwestResponse,
};
use serde::{
    de::{
        DeserializeOwned,
        Error,
    },
    Deserialize,
    Deserializer,
    Serialize,
};

static VALIDATOR_SET_ENDPOINT: &str = "/cosmos/base/tendermint/v1beta1/validatorsets/";

#[derive(Serialize, Debug)]
pub struct EmptyRequest {}

#[derive(Deserialize, Serialize, Debug)]
pub struct ValidatorSet {
    pub block_height: String,
    pub validators: Vec<Validator>,
}

impl ValidatorSet {
    /// returns the proposer given the current set by ordering the validators by proposer priority.
    /// the validator with the highest proposer priority is the proposer.
    /// TODO: could there ever be two validators with the same priority?
    pub(crate) fn get_proposer(&self) -> eyre::Result<Validator> {
        self.validators
            .iter()
            .max_by(|v1, v2| v1.proposer_priority.cmp(&v2.proposer_priority))
            .cloned()
            .ok_or_else(|| eyre::eyre!("no proposer found"))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Validator {
    pub address: String,
    pub pub_key: KeyWithType,
    #[serde(deserialize_with = "deserialize_int_from_str")]
    pub voting_power: u64,
    #[serde(deserialize_with = "deserialize_int_from_str")]
    pub proposer_priority: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct KeyWithType {
    #[serde(rename = "@type")]
    pub key_type: String,
    pub key: Base64String,
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
        let validator_set = self.get_validator_set(height).await?;
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

fn deserialize_int_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + serde::Deserialize<'de>,
    <T as FromStr>::Err: Display,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse::<T>().map_err(D::Error::custom)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn validator_serialize() {
        let validator_str = r#"
              {
                "address": "metrovalcons1hdu2nzhcyfnhaj9tfrdlekfnfwx895mk83d322",
                "pub_key": {
                  "@type": "/cosmos.crypto.ed25519.PubKey",
                  "key": "MdfFS4MH09Og5y+9SVxpJRqUnZkDGfnPjdyx4qM2Vng="
                },
                "voting_power": "5000",
                "proposer_priority": "0"
              }"#;

        let validator = serde_json::from_str::<Validator>(validator_str).unwrap();
        assert_eq!(validator.voting_power, 5000);
        assert_eq!(validator.proposer_priority, 0);
    }
}
