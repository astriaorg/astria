use std::time::Duration;

use eyre::WrapErr as _;
use reqwest::{
    Client,
    Response as ReqwestResponse,
};
use serde::{
    de::DeserializeOwned,
    Serialize,
};

use crate::types::*;

static BLOCK_ENDPOINT: &str = "/cosmos/base/tendermint/v1beta1/blocks/";
static LATEST_BLOCK_ENDPOINT: &str = "/cosmos/base/tendermint/v1beta1/blocks/latest";

pub struct SequencerClient {
    endpoint: String,
    http_client: Client,
}

impl SequencerClient {
    pub fn new(endpoint: String) -> eyre::Result<Self> {
        let http_client: Client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        Ok(Self {
            endpoint,
            http_client,
        })
    }

    pub async fn get_latest_block(&self) -> eyre::Result<BlockResponse> {
        let endpoint: String = format!("{}{}", self.endpoint, LATEST_BLOCK_ENDPOINT);
        self.do_get::<EmptyRequest, BlockResponse>(endpoint, None)
            .await
            .wrap_err("failed getting latest block")
    }

    pub async fn get_block(&self, height: u64) -> eyre::Result<BlockResponse> {
        let endpoint: String = format!("{}{}{}", self.endpoint, BLOCK_ENDPOINT, height);
        self.do_get::<EmptyRequest, BlockResponse>(endpoint, None)
            .await
            .wrap_err_with(|| format!("failed getting block at height `{height}`"))
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
