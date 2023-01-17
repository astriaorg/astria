use std::time::Duration;

// use hex;
use reqwest::{Client as ReqwestClient, Response as ReqwestResponse};
use serde::{Deserialize, Serialize};

mod error;

// TODO - organization
// const BALANCE_ENDPOINT: &str = "/balance";
// const HEADER_ENDPOINT: &str = "/header";
// const NAMESPACED_SHARES_ENDPOINT: &str = "/namespaced_shares";
// const NAMESPACED_DATA_ENDPOINT: &str = "/namespaced_data";
const SUBMIT_PFD_ENDPOINT: &str = "/submit_pfd";
// const SUBMIT_TX_ENDPOINT: &str = "/submit_tx";

pub struct Client {
    /// The url of the Celestia node
    base_url: String,

    /// An http client for making http requests
    http_client: ReqwestClient,
}

#[derive(Serialize, Debug)]
struct SubmitPFDRequest {
    namespace_id: String,
    data: String,
    fee: i64,
    gas_limit: u64,
}


#[derive(Deserialize, Debug)]
pub struct SubmitPFDResponse {
    height: Option<i64>,
    txhash: String,
    data: Option<String>,
    raw_log: Option<String>,
    events: Option<Vec<Event>>,
    logs: Option<Vec<Log>>,
    code: Option<i64>,
    codespace: Option<String>,
    gas_wanted: Option<i64>,
    gas_used: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct Event {
    #[serde(rename = "type")]
    type_field: String,
    attributes: Vec<Attribute>,
}

#[derive(Deserialize, Debug)]
pub struct Log {
    msg_index: i64,
    events: Option<Vec<Event>>,
}

#[derive(Deserialize, Debug)]
pub struct Attribute {
    key: String,
    value: String,
    index: Option<bool>,
}

impl Client {
    /// Creates a new client
    ///
    /// # Arguments
    ///
    /// * `base_url` - A string that holds the base url we want to communicate with
    pub fn new(base_url: String) -> Result<Self, error::ClientError> {
        let http_client: ReqwestClient;
        let http_client_res: Result<ReqwestClient, reqwest::Error> = ReqwestClient::builder()
            .timeout(Duration::from_secs(5))
            .build();

        if http_client_res.is_err() {
            let error_string = http_client_res.unwrap_err().to_string();
            return Err(error::ClientError::Http(error_string));
        }

        http_client = http_client_res.unwrap();

        Ok(Self {
            base_url,
            http_client,
        })
    }

    #[tokio::main]
    pub async fn submit_pfd(
        &self,
        namespace_id: [u8; 8],
        data: Vec<u8>,
        fee: i64,
        gas_limit: u64,
    ) -> Result<SubmitPFDResponse, reqwest::Error> {
        // convert namespace and data to hex
        let namespace_id: String = hex::encode(namespace_id);
        let data: String = hex::encode(data);

        let body = SubmitPFDRequest {
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

        // FIXME - remove after developing
        // let response_text = response.text().await.unwrap();
        // println!("{}", response_text);
        // let response: SubmitPFDResponse = serde_json::from_str::<SubmitPFDResponse>(&response_text).unwrap();
        // println!("{:#?}", response);

        let response = response
            .json::<SubmitPFDResponse>()
            .await?;

        Ok(response)
    }

    #[tokio::main]
    pub async fn namespaced_data(&self, namespace_id: [u8; 8], height: u64) {
        println!("{:#?}", namespace_id);
        println!("{}", height);
        todo!();
    }
    //
    // pub async fn submit_tx() {
    //     todo!();
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_creates_client() {
        let base_url = String::from("http://localhost:26659");
        let client: Client = Client::new(base_url).unwrap();
        assert_eq!(&client.base_url, "http://localhost:26659");
    }
}
