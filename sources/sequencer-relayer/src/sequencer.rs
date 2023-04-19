use eyre::WrapErr as _;
use reqwest::{Client, Response as ReqwestResponse};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

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

#[cfg(test)]
mod test {
    use crate::base64_string::Base64String;
    use bech32::{self, FromBase32, Variant};

    #[test]
    fn test_decode_validator_address() {
        // when we get a validator address from a block, it's in base64
        let validator_address_from_block =
            Base64String::from_string("ehH7+Y2s/jspdUgMZ8fy8a1BqUo=".to_string()).unwrap();

        // validator address from bech32; retrieved with `metro tendermint show-address`
        let (hrp, data, variant) =
            bech32::decode("metrovalcons10gglh7vd4nlrk2t4fqxx03lj7xk5r22202rwyt").unwrap();
        assert_eq!(hrp, "metrovalcons");
        assert_eq!(
            Vec::<u8>::from_base32(&data).unwrap(),
            validator_address_from_block.0
        );
        assert_eq!(variant, Variant::Bech32);
    }
}
