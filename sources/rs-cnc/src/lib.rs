use std::collections::HashMap;
use std::time::Duration;

use hex;
use reqwest::{Client as ReqwestClient, Response as ReqwestResponse};
use serde::Serialize;

mod error;

// TODO - organization
// const BALANCE_ENDPOINT: &str = "/balance";
// const HEADER_ENDPOINT: &str = "/header";
// const NAMESPACED_SHARES_ENDPOINT: &str = "/namespaced_shares";
// const NAMESPACED_DATA_ENDPOINT: &str = "/namespaced_data";
const SUBMIT_PFD_ENDPOINT: &str = "/submit_pdf";
// const SUBMIT_TX_ENDPOINT: &str = "/submit_tx";

pub struct Client {
    /// The url of the Celestia node
    base_url: String,

    /// An http client for making http requests
    http_client: ReqwestClient,
}

#[derive(Serialize)]
struct SubmitPFDRequest {
    namespace_id: String,
    data: String,
    fee: i64,
    gas_limit: u64,
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
        namespace_id: String,
        data: String,
        fee: i64,
        gas_limit: u64,
    ) -> Result<(), reqwest::Error> {
        // convert namespace and data to hex
        // let namespace_id: String = format!("{:x}", namespace_id);
        // let data: String = hex::encode(data);
        let namespace_id: String = String::from("0c204d39600fddd3");
        let data: String = String::from("f1f20ca8007e910a3bf8b2e61da0f26bca07ef78717a6ea54165f5");

        let body = SubmitPFDRequest {
            namespace_id,
            data,
            fee,
            gas_limit,
        };
        let body = serde_json::to_string(&body).unwrap();

        let url: String = format!("{}{}", self.base_url, SUBMIT_PFD_ENDPOINT);

        let response: ReqwestResponse = self
            .http_client
            .post(url)
            .json(&body)
            .send()
            .await?;
        let js: HashMap<String, String> = response
            .json::<HashMap<String, String>>()
            .await?;

        // match response.status() {
        //     reqwest::StatusCode::OK => {
        //         match response.json::<HashMap<String, String>>().await {
        //             Ok(parsed) => println!("Success! {:?}", parsed),
        //             Err(_) => println!("Hm, the response didn't match the shape we expected."),
        //         };
        //     }
        //     reqwest::StatusCode::UNAUTHORIZED => {
        //         println!("Need to grab a new token");
        //     }
        //     status => {
        //         println!("Something unexpected happened. {}", status);
        //     }
        // };

        Ok(())
    }

    // pub async fn namespaced_data(&self, namespace_id: u8, height: u64) {}

    // pub async fn submit_tx() {}
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
