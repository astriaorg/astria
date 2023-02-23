use anyhow::{anyhow, Error};
use reqwest::{Client, Response as ReqwestResponse};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

use crate::types::*;

static LATEST_BLOCK_ENDPOINT: &str = "/cosmos/base/tendermint/v1beta1/blocks/latest";

pub struct SequencerClient {
    endpoint: String,
    http_client: Client,
}

impl SequencerClient {
    pub fn new(endpoint: String) -> Result<Self, Error> {
        let http_client: Client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        Ok(Self {
            endpoint,
            http_client,
        })
    }

    pub async fn get_latest_block(&self) -> Result<LatestBlockResponse, Error> {
        let endpoint: String = format!("{}{}", self.endpoint, LATEST_BLOCK_ENDPOINT);
        self.do_get::<EmptyRequest, LatestBlockResponse>(endpoint, None)
            .await
    }

    async fn do_get<Req: Serialize + Sized, Resp: DeserializeOwned>(
        &self,
        endpoint: String,
        req: Option<Req>,
    ) -> Result<Resp, Error> {
        let response: ReqwestResponse = self.http_client.get(&endpoint).json(&req).send().await?;
        response
            .error_for_status()
            .map_err(|e| anyhow!(e))?
            .json::<Resp>()
            .await
            .map_err(|e| anyhow!(e))
    }
}

#[cfg(test)]
mod test {
    use super::SequencerClient;

    #[tokio::test]
    async fn test_get_latest_block() {
        let cosmos_endpoint = "http://localhost:1317".to_string();
        let client = SequencerClient::new(cosmos_endpoint).unwrap();
        let resp = client.get_latest_block().await.unwrap();
        println!("{:?}", resp);
    }
}
