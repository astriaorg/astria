use std::hash::Hash;
use std::time::Duration;

use bytes::Bytes;
use eyre::{bail, WrapErr as _};
use reqwest::{Client, Response as ReqwestResponse};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::types::Base64String;

pub mod types;

// TODO - organize
const NAMESPACED_DATA_ENDPOINT: &str = "/namespaced_data";
const NAMESPACED_SHARES_ENDPOINT: &str = "/namespaced_shares";
const SUBMIT_PFD_ENDPOINT: &str = "/submit_pfd";

#[derive(Serialize, Debug)]
struct PayForDataRequest {
    namespace_id: String,
    data: String,
    fee: i64,
    gas_limit: u64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct PayForDataResponse {
    /// The block height.
    pub height: Option<u64>,
    /// The transaction hash.
    pub txhash: Option<String>,
    /// Result bytes, if any.
    data: Option<String>,
    /// The output of the application's logger (raw string). May be non-deterministic.
    raw_log: Option<String>,
    ///
    events: Option<Vec<Event>>,
    /// The output of the application's logger (typed). May be non-deterministic.
    logs: Option<Vec<Log>>,
    /// Namespace for the code.
    codespace: Option<String>,
    /// Response code.
    code: Option<u64>,
    /// Amount of gas requested for transaction.
    gas_wanted: Option<u64>,
    /// Amount of gas consumed by transaction.
    gas_used: Option<u64>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Event {
    #[serde(rename = "type")]
    type_field: String,
    attributes: Vec<Attribute>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Log {
    msg_index: u64,
    events: Option<Vec<Event>>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Attribute {
    key: String,
    value: String,
    index: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NamespacedSharesResponse {
    pub height: u64,
    pub shares: Vec<Base64String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NamespacedDataResponse {
    pub height: Option<u64>,
    pub data: Option<Vec<Base64String>>,
}

// allows NamespacedDataResponse to be used as a HashMap key
impl Hash for NamespacedDataResponse {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.height.hash(state);
    }
}

impl Eq for NamespacedDataResponse {}

// allows one to compare NamespacedDataResponses with assert_eq!
impl PartialEq for NamespacedDataResponse {
    fn eq(&self, other: &NamespacedDataResponse) -> bool {
        self.height == other.height
    }
}

#[derive(Debug)]
pub struct CelestiaNodeClient {
    /// The url of the Celestia node.
    base_url: String,

    /// An http client for making http requests.
    http_client: Client,
}

impl CelestiaNodeClient {
    /// Creates a `CelestiaNodeClientBuilder` to configure a `CelestiaNodeClient`.
    pub fn builder() -> CelestiaNodeClientBuilder {
        CelestiaNodeClientBuilder::new()
    }
    /// Creates a new client
    ///
    /// # Arguments
    ///
    /// * `base_url` - A string that holds the base url we want to communicate with
    pub fn new(base_url: String) -> eyre::Result<Self> {
        let http_client: Client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .wrap_err("failed initializing http client")?;

        Ok(Self {
            base_url,
            http_client,
        })
    }

    pub async fn submit_pay_for_data(
        &self,
        namespace_id: &str,
        data: &Bytes,
        fee: i64,
        gas_limit: u64,
    ) -> eyre::Result<PayForDataResponse> {
        let data: String = hex::encode(data);

        let body = PayForDataRequest {
            namespace_id: namespace_id.to_owned(),
            data,
            fee,
            gas_limit,
        };

        let url: String = format!("{}{}", self.base_url, SUBMIT_PFD_ENDPOINT);

        let response: ReqwestResponse = self
            .http_client
            .post(url)
            .json(&body)
            .send()
            .await
            .wrap_err("failed sending POST request to endpoint")?;
        let response = response
            .error_for_status()
            .wrap_err("server responded with error code")?
            .json::<PayForDataResponse>()
            .await
            .wrap_err("failed reading JSON response from server")?;

        Ok(response)
    }

    pub async fn namespaced_shares(
        &self,
        namespace_id: &str,
        height: u64,
    ) -> eyre::Result<NamespacedSharesResponse> {
        let url = format!(
            "{}{}/{}/height/{}",
            self.base_url, NAMESPACED_SHARES_ENDPOINT, namespace_id, height,
        );

        let response = self
            .do_get::<NamespacedSharesResponse>(url)
            .await
            .wrap_err("failed getting namespaced shares from server")?;
        Ok(response)
    }

    pub async fn namespaced_data(
        &self,
        namespace_id: &str,
        height: u64,
    ) -> eyre::Result<NamespacedDataResponse> {
        let url = format!(
            "{}{}/{}/height/{}",
            self.base_url, NAMESPACED_DATA_ENDPOINT, namespace_id, height,
        );

        let response = self
            .do_get::<NamespacedDataResponse>(url)
            .await
            .wrap_err("failed getting namespaced data from server")?;
        Ok(response)
    }

    async fn do_get<Resp: DeserializeOwned>(&self, endpoint: String) -> eyre::Result<Resp> {
        let response = self
            .http_client
            .get(&endpoint)
            .send()
            .await
            .wrap_err("failed sending GET request to endpoint")?;
        response
            .error_for_status()
            .wrap_err("server responded with error code")?
            .json::<Resp>()
            .await
            .wrap_err("failed reading JSON response from server")
    }
}

/// A `CelestiaNodeClientBuilder` can be used to create a `CelstiaNodeClient`.
#[derive(Debug)]
pub struct CelestiaNodeClientBuilder {
    base_url: Option<String>,
    http_client: Option<reqwest::Client>,
}

impl CelestiaNodeClientBuilder {
    /// Sets the base URL used by this client.
    pub fn base_url<T: Into<String>>(self, base_url: T) -> Self {
        Self {
            base_url: Some(base_url.into()),
            ..self
        }
    }

    /// Sets the http_client used by this client.
    ///
    /// Default is the same as `reqwest::Client::default`
    pub fn http_client(self, http_client: Client) -> Self {
        Self {
            http_client: Some(http_client),
            ..self
        }
    }

    /// Returns a `CelestiaNodeClient` that uses this `CelestiaNodeClientBuilder` as config.
    ///
    /// # Errors
    ///
    /// + returns an error if `base_url` is not set.
    pub fn build(self) -> eyre::Result<CelestiaNodeClient> {
        let Self {
            base_url,
            http_client,
        } = self;
        let Some(base_url) = base_url else {
            bail!("base_url on CelestiaNodeClientBuilder not set");
        };
        let http_client = http_client.unwrap_or_default();

        Ok(CelestiaNodeClient {
            base_url,
            http_client,
        })
    }

    /// Returns a new `CelestiaNodeClientBuilder`.
    pub fn new() -> Self {
        Self {
            base_url: None,
            http_client: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_creates_client() {
        let base_url = String::from("http://localhost:26659");
        let client: CelestiaNodeClient = CelestiaNodeClient::new(base_url).unwrap();
        assert_eq!(&client.base_url, "http://localhost:26659");
    }

    #[test]
    fn constructing_client_without_base_url_is_err() {
        let err = CelestiaNodeClient::builder().build().unwrap_err();
        assert_eq!(
            "base_url on CelestiaNodeClientBuilder not set",
            err.to_string()
        );
    }
}
