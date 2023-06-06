//! Rust interface to the Celestia Node API using JsonRPC
use http::header::{HeaderMap, HeaderValue};

pub mod error;
pub mod rpc_impl;
pub(crate) mod serde;

use ::serde::{Deserialize, Serialize};
pub use error::{BuildError, RpcError};

pub struct CelestiaHttpClient {
    inner: jsonrpsee::http_client::HttpClient,
}

impl CelestiaHttpClient {
    pub async fn submit_pay_for_blob(
        &self,
        fee: String,
        gas_limit: u64,
        blobs: &[Blob],
    ) -> Result<SubmitPayForBlobResponse, RpcError> {
        rpc_impl::state::StateClient::submit_pay_for_blob(&self.inner, fee, gas_limit, blobs)
            .await
            .map_err(RpcError::from_jsonrpsee)
    }

    pub async fn blob_submit(&self, blobs: &[BlobWithCommitment]) -> Result<u64, RpcError> {
        rpc_impl::blob::BlobClient::submit()
    }

    async fn get(
        &self,
        height: u64,
        namespace_id: Vec<u8>,
        commitment: &Commitment,
    ) -> Result<BlobWithCommitment, RpcError> {
    }

    async fn get_all(
        &self,
        height: u64,
        namespace_ids: &[NamespaceId],
    ) -> Result<Vec<BlobWithCommitment>, RpcError> {
    }

    async fn get_proof(
        &self,
        height: u64,
        namespace_id: NamespaceId,
        commitment: &Commitment,
    ) -> Result<Vec<Proof>, Error> {
    }

    async fn included(
        &self,
        height: u64,
        namespace_id: NamespaceId,
        proofs: &[Proof],
        commitment: &Commitment,
    ) -> Result<bool, Error> {
    }
}

impl CelestiaHttpClient {
    pub fn builder() -> CelestiaHttpClientBuilder {
        CelestiaHttpClientBuilder::default()
    }
}

#[derive(Debug, Default)]
pub struct CelestiaHttpClientBuilder {
    bearer_token: Option<String>,
    endpoint: Option<String>,
}

impl CelestiaHttpClientBuilder {
    pub fn build(self) -> Result<CelestiaHttpClient, BuildError> {
        let Self {
            bearer_token,
            endpoint,
        } = self;
        let Some(bearer_token) = bearer_token else {
            return Err(BuildError::field_not_set("bearer_token"));
        };
        let Some(endpoint) = endpoint else {
            return Err(BuildError::field_not_set("endpoint"));
        };
        let mut header_map = HeaderMap::new();
        header_map.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {bearer_token}"))
                .map_err(BuildError::invalid_bearer_token)?,
        );
        let inner = jsonrpsee::http_client::HttpClientBuilder::default()
            .set_headers(header_map)
            .build(endpoint)
            .map_err(BuildError::inner_client)?;
        let client = CelestiaHttpClient { inner };
        Ok(client)
    }

    pub fn bearer_token(self, bearer_token: impl ToString) -> Self {
        Self {
            bearer_token: Some(bearer_token.to_string()),
            ..self
        }
    }

    pub fn endpoint(self, endpoint: impl ToString) -> Self {
        Self {
            endpoint: Some(endpoint.to_string()),
            ..self
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Blob {
    #[serde(with = "crate::serde::Base64Standard")]
    pub namespace_id: Vec<u8>,
    #[serde(with = "crate::serde::Base64Standard")]
    pub data: Vec<u8>,
}

#[derive(Deserialize, Debug)]
pub struct SubmitPayForBlobResponse {
    pub height: u64,
    pub txhash: String,
    pub data: String,
    pub raw_log: serde_json::Value,
    pub logs: serde_json::Value,
    pub gas_wanted: u64,
    pub gas_used: u64,
    pub events: serde_json::Value,
}
