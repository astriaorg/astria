use hex;
use reqwest::Response;
use serde::{Deserialize, Serialize};

// TODO - organization
const SUBMIT_PFD_ENDPOINT: &str = "/submit_pdf";

pub struct Client {
    /// The url of the Celestia node
    base_url: String,

    /// An http client for making http requests
    http_client: reqwest::Client,
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
    pub fn new(base_url: String) -> Self {
        let http_client = reqwest::Client::new();
        Self {
            base_url,
            http_client,
        }
    }

    #[tokio::main]
    pub async fn submit_pfd(
        &self,
        namespace_id: u8,
        data: String,
        fee: i64,
        gas_limit: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let namespace_id: String = format!("{:x}", namespace_id);
        let data: String = hex::encode(data);
        let request = SubmitPFDRequest {
            namespace_id,
            data,
            fee,
            gas_limit,
        };
        let request = serde_json::to_string(&request).unwrap();

        let response: Response = self
            .http_client
            .post("{self.base_url}{SUBMIT_PFD_ENDPOINT}")
            .json(&request)
            .send()
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_creates_client() {
        let base_url = String::from("http://localhost:26659");
        let client: Client = Client::new(base_url);
        assert_eq!(&client.base_url, "http://localhost:26659");
    }
}
