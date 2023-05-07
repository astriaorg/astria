use std::time::Duration;
use std::{borrow::Cow, hash::Hash};

use bytes::Bytes;
use eyre::{bail, WrapErr as _};
use reqwest::{Client, IntoUrl, Response as ReqwestResponse, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::types::Base64String;

pub mod types;

// TODO - organize
const NAMESPACED_DATA_ENDPOINT: &str = "namespaced_data/";
const NAMESPACED_SHARES_ENDPOINT: &str = "namespaced_shares/";
const SUBMIT_PFD_ENDPOINT: &str = "submit_pfd";

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
struct CelestiaNodeEndpoints {
    namespaced_data: Url,
    namespaced_shares: Url,
    submit_pfd: Url,
}

impl CelestiaNodeEndpoints {
    fn try_from_url(url: &Url) -> eyre::Result<Self> {
        let namespaced_data = url
            .join(NAMESPACED_DATA_ENDPOINT)
            .wrap_err("failed creating URL for namespaced data endpoint")?;
        let namespaced_shares = url
            .join(NAMESPACED_SHARES_ENDPOINT)
            .wrap_err("failed creating URL for namespaced shares endpoint")?;
        let submit_pfd = url
            .join(SUBMIT_PFD_ENDPOINT)
            .wrap_err("failed creating URL for submit pfd endpoint")?;

        Ok(Self {
            namespaced_shares,
            namespaced_data,
            submit_pfd,
        })
    }

    fn to_namespaced_data_url(&self, namespace_id: &str, height: u64) -> eyre::Result<Url> {
        self.namespaced_data
            .join(&format!("{namespace_id}/height/{height}"))
            .wrap_err("failed constructing namespaced data request URL")
    }

    fn to_namespaced_shares_url(&self, namespace_id: &str, height: u64) -> eyre::Result<Url> {
        self.namespaced_shares
            .join(&format!("{namespace_id}/height/{height}"))
            .wrap_err("failed constructing namespaced shares request URL")
    }
}

#[derive(Debug)]
pub struct CelestiaNodeClient {
    /// The url of the Celestia node.
    base_url: Url,

    /// An http client for making http requests.
    http_client: Client,

    /// The various endpoints used by this node
    endpoints: CelestiaNodeEndpoints,
}

impl CelestiaNodeClient {
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Creates a `CelestiaNodeClientBuilder` to configure a `CelestiaNodeClient`.
    pub fn builder() -> CelestiaNodeClientBuilder {
        CelestiaNodeClientBuilder::new()
    }
    /// Creates a new client
    ///
    /// # Arguments
    ///
    /// * `base_url` - A string that holds the base url we want to communicate with
    pub fn new(base_url: &str) -> eyre::Result<Self> {
        let http_client: Client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .wrap_err("failed initializing http client")?;
        Self::builder()
            .base_url(base_url)?
            .http_client(http_client)
            .build()
            .wrap_err("failed constructing celestia node client")
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

        let response: ReqwestResponse = self
            .http_client
            .post(self.endpoints.submit_pfd.clone())
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
        let url = self
            .endpoints
            .to_namespaced_shares_url(namespace_id, height)
            .wrap_err("failed constructing URL for namespaced shares endpoint")?;

        let response = self
            .do_get(url)
            .await
            .wrap_err("failed getting namespaced shares from server")?;
        Ok(response)
    }

    pub async fn namespaced_data(
        &self,
        namespace_id: &str,
        height: u64,
    ) -> eyre::Result<NamespacedDataResponse> {
        let url = self
            .endpoints
            .to_namespaced_data_url(namespace_id, height)
            .wrap_err("failed constructing URL for namspaced data endpoint")?;

        let response = self
            .do_get(url)
            .await
            .wrap_err("failed getting namespaced data from server")?;
        Ok(response)
    }

    async fn do_get<Resp: DeserializeOwned, T: IntoUrl>(&self, url: T) -> eyre::Result<Resp> {
        let response = self
            .http_client
            .get(url)
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
    base_url: Option<Url>,
    http_client: Option<reqwest::Client>,
}

impl CelestiaNodeClientBuilder {
    /// Sets the base URL used by this client.
    pub fn base_url<T: AsRef<str>>(self, base_url: T) -> eyre::Result<Self> {
        let base_url = base_url.as_ref();
        let base_url = if &base_url[base_url.len()..] == "/" {
            Cow::Borrowed(base_url)
        } else {
            let mut s = base_url.to_string();
            s.push('/');
            Cow::Owned(s)
        };
        let base_url = Url::parse(&base_url).wrap_err("failed parsing provided string as URL")?;
        Ok(Self {
            base_url: Some(base_url),
            ..self
        })
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

        let endpoints = CelestiaNodeEndpoints::try_from_url(&base_url)
            .wrap_err("failed constructing endpoints from base URL")?;

        Ok(CelestiaNodeClient {
            base_url,
            http_client,
            endpoints,
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
    use reqwest::Url;

    use super::{
        CelestiaNodeClient, CelestiaNodeEndpoints, NAMESPACED_DATA_ENDPOINT,
        NAMESPACED_SHARES_ENDPOINT, SUBMIT_PFD_ENDPOINT,
    };

    #[test]
    fn constructing_client_without_base_url_is_err() {
        let err = CelestiaNodeClient::builder().build().unwrap_err();
        assert_eq!(
            "base_url on CelestiaNodeClientBuilder not set",
            err.to_string()
        );
    }

    #[test]
    fn base_url_is_made_into_proper_base_with_slash() {
        let url_without_trailing_slash = "http://localdev.me/celestia";
        let url_with_trailing_slash = format!("{url_without_trailing_slash}/");

        let client = CelestiaNodeClient::new("http://localdev.me/celestia").unwrap();
        assert_eq!(&url_with_trailing_slash, client.base_url.as_str());
    }

    #[test]
    fn all_endpoints_are_proper_bases() {
        let base_url = Url::parse("http://localdev.me/celestia/").unwrap();
        let endpoints = CelestiaNodeEndpoints::try_from_url(&base_url).unwrap();
        let namespaced_data = format!("{base_url}{NAMESPACED_DATA_ENDPOINT}");
        let namespaced_shares = format!("{base_url}{NAMESPACED_SHARES_ENDPOINT}");
        let submit_pfd = format!("{base_url}{SUBMIT_PFD_ENDPOINT}");
        assert_eq!(namespaced_data, endpoints.namespaced_data.as_str());
        assert_eq!(namespaced_shares, endpoints.namespaced_shares.as_str());
        assert_eq!(submit_pfd, endpoints.submit_pfd.as_str());
    }

    #[test]
    fn namespaced_data_request_url_is_as_expected() {
        let base_url = Url::parse("http://localdev.me/celestia/").unwrap();
        let endpoints = CelestiaNodeEndpoints::try_from_url(&base_url).unwrap();
        let namespace_id = "123";
        let height = 4;
        let expected_url = "http://localdev.me/celestia/namespaced_data/123/height/4";
        let actual_url = endpoints
            .to_namespaced_data_url(namespace_id, height)
            .unwrap();
        assert_eq!(expected_url, actual_url.as_str());
    }

    #[test]
    fn namespaced_shares_request_url_is_as_expected() {
        let base_url = Url::parse("http://localdev.me/celestia/").unwrap();
        let endpoints = CelestiaNodeEndpoints::try_from_url(&base_url).unwrap();
        let namespace_id = "123";
        let height = 4;
        let expected_url = "http://localdev.me/celestia/namespaced_shares/123/height/4";
        let actual_url = endpoints
            .to_namespaced_shares_url(namespace_id, height)
            .unwrap();
        assert_eq!(expected_url, actual_url.as_str());
    }

    #[test]
    fn submit_pfd_is_as_expected() {
        let base_url = Url::parse("http://localdev.me/celestia/").unwrap();
        let endpoints = CelestiaNodeEndpoints::try_from_url(&base_url).unwrap();
        let expected_url = "http://localdev.me/celestia/submit_pfd";
        assert_eq!(expected_url, endpoints.submit_pfd.as_str());
    }
}
