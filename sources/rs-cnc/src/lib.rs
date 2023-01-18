use std::time::Duration;
use bytes::Bytes;

use reqwest::{Client, Response as ReqwestResponse};
use serde::{Deserialize, Serialize};

mod error;

// TODO - organize
const NAMESPACED_DATA_ENDPOINT: &str = "/namespaced_data";
const SUBMIT_PFD_ENDPOINT: &str = "/submit_pfd";

pub struct CelestiaNodeClient {
    /// The url of the Celestia node.
    base_url: String,

    /// An http client for making http requests.
    http_client: Client,
}

#[derive(Serialize, Debug)]
struct PayForDataRequest {
    namespace_id: String,
    data: String,
    fee: i64,
    gas_limit: u64,
}

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

#[derive(Deserialize, Debug)]
pub struct Event {
    #[serde(rename = "type")]
    type_field: String,
    attributes: Vec<Attribute>,
}

#[derive(Deserialize, Debug)]
pub struct Log {
    msg_index: u64,
    events: Option<Vec<Event>>,
}

#[derive(Deserialize, Debug)]
pub struct Attribute {
    key: String,
    value: String,
    index: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct NamespacedDataResponse {
    pub height: Option<u64>,
    pub data: Option<Vec<String>>,
}

impl CelestiaNodeClient {
    /// Creates a new client
    ///
    /// # Arguments
    ///
    /// * `base_url` - A string that holds the base url we want to communicate with
    pub fn new(base_url: String) -> Result<Self, error::CelestiaNodeClientError> {
        let http_client: Client;
        let http_client_res: Result<Client, reqwest::Error> = Client::builder()
            .timeout(Duration::from_secs(5))
            .build();

        if http_client_res.is_err() {
            let error_string = http_client_res.unwrap_err().to_string();
            return Err(error::CelestiaNodeClientError::HttpClient(error_string));
        }

        http_client = http_client_res.unwrap();

        Ok(Self {
            base_url,
            http_client,
        })
    }

    #[tokio::main]
    pub async fn submit_pay_for_data(
        &self,
        namespace_id: &[u8; 8],
        data: &Bytes,
        fee: i64,
        gas_limit: u64,
    ) -> Result<PayForDataResponse, reqwest::Error> {
        let namespace_id: String = hex::encode(namespace_id);
        let data: String = hex::encode(data);

        let body = PayForDataRequest {
            namespace_id,
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
            .await?;

        let response = response
            .json::<PayForDataResponse>()
            .await?;

        Ok(response)
    }

    #[tokio::main]
    pub async fn namespaced_data(
        &self,
        namespace_id: [u8; 8],
        height: u64,
    ) -> Result<NamespacedDataResponse, reqwest::Error> {
        let namespace_id: String = hex::encode(namespace_id);
        let url = format!(
            "{}{}/{}/height/{}",
            self.base_url,
            NAMESPACED_DATA_ENDPOINT,
            namespace_id,
            height,
        );

        let response: ReqwestResponse = self
            .http_client
            .get(url)
            .send()
            .await?;

        let response = response
            .json::<NamespacedDataResponse>()
            .await?;

        Ok(response)
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
}
