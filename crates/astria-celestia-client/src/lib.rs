//! Rust interface to the Celestia Node API using JsonRPC
use base64::{
    prelude::BASE64_STANDARD,
    Engine as _,
};
use http::header::{
    HeaderMap,
    HeaderValue,
};

pub mod error;
pub mod rpc_impl;
pub(crate) mod serde;

pub use error::{
    BuildError,
    RpcError,
};
// pub use rpc_impl::daser;
// pub use rpc_impl::daser::DaserClient;
// pub use rpc_impl::fraud::FraudClient;
// use rpc_impl::header::HeaderClient;
// pub use rpc_impl::node::NodeClient;
// pub use rpc_impl::p2p::P2pClient;
// pub use rpc_impl::share::ShareClient;
// use rpc_impl::state::StateClient;
use rpc_impl::state::SubmitPayForBlobResponse;

pub struct CelestiaHttpClient {
    inner: jsonrpsee::http_client::HttpClient,
}

impl CelestiaHttpClient {
    pub async fn submit_pay_for_blob(
        &self,
        namespace: [u8; 8],
        data: &[u8],
        fee: String,
        gas_limit: u64,
    ) -> Result<SubmitPayForBlobResponse, RpcError> {
        let namespace = BASE64_STANDARD.encode(namespace);
        let data = BASE64_STANDARD.encode(data);
        rpc_impl::state::StateClient::submit_pay_for_blob(
            &self.inner,
            namespace,
            data,
            fee,
            gas_limit,
        )
        .await
        .map_err(RpcError::from_jsonrpsee)
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
        let client = CelestiaHttpClient {
            inner,
        };
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
