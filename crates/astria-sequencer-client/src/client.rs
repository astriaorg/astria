use std::time::Duration;

use astria_sequencer::{
    accounts::types::{
        Address,
        Balance,
        Nonce,
    },
    transaction::signed,
};
use borsh::BorshDeserialize;
use eyre::{
    self,
    WrapErr as _,
};
use reqwest::{
    self,
    ClientBuilder,
};
use serde::Deserialize;

const DEFAULT_TENDERMINT_BASE_URL: &str = "http://localhost:26657";

/// Tendermint client which is used to interact with the Sequencer node.
pub struct Client {
    pub client: reqwest::Client,
    pub base_url: String,
}

impl Client {
    pub fn new(base_url: &str) -> eyre::Result<Self> {
        let http_client = ClientBuilder::new()
            .timeout(Duration::from_secs(5))
            .build()
            .wrap_err("failed initializing http client")?;

        Ok(Client {
            client: http_client,
            base_url: base_url.to_string(),
        })
    }

    pub fn default() -> eyre::Result<Self> {
        Self::new(DEFAULT_TENDERMINT_BASE_URL)
    }

    pub async fn get_balance(&self, address: &Address, height: u64) -> eyre::Result<Balance> {
        let url = format!(
            "{}/abci_query?path=%2Faccounts%2Fbalance%2F{}&data=IHAVENOIDEA&height={}&prove=false",
            self.base_url,
            address.to_string(),
            height
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .wrap_err("failed to send request")?;

        let response = response
            .error_for_status()
            .wrap_err("server responded with error code")?
            .json::<QueryResponse>()
            .await
            .wrap_err("failed reading JSON response from server")?;

        let balance = Balance::try_from_slice(
            &hex::decode(response.response.value)
                .wrap_err("failed to decode query response value hex strng")?,
        )
        .wrap_err("failed to deserialize balance bytes")?;
        Ok(balance)
    }

    pub async fn get_nonce(&self, address: &Address, height: u64) -> eyre::Result<Nonce> {
        let url = format!(
            "{}/abci_query?path=%2Faccounts%2Fnonce%2F{}&data=IHAVENOIDEA&height={}&prove=false",
            self.base_url,
            address.to_string(),
            height
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .wrap_err("failed to send request")?;

        let response = response
            .error_for_status()
            .wrap_err("server responded with error code")?
            .json::<QueryResponse>()
            .await
            .wrap_err("failed reading JSON response from server")?;

        let nonce = Nonce::try_from_slice(
            &hex::decode(response.response.value)
                .wrap_err("failed to decode query response value hex strng")?,
        )
        .wrap_err("failed to deserialize nonce bytes")?;
        Ok(nonce)
    }

    /// Submits the given transaction to the Sequencer node.
    /// This method blocks until the transaction is checked, but not until it's committed.
    pub async fn submit_transaction_sync(
        &self,
        tx: signed::Transaction,
    ) -> eyre::Result<SubmitTransactionResponse> {
        let url = format!("{}/broadcast_tx_sync", self.base_url);
        let tx_bytes = tx.to_proto();
        let tx_hex = hex::encode(&tx_bytes);
        let params = [("tx", tx_hex)];
        let response = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .wrap_err("failed to send transaction")?;

        let response = response
            .error_for_status()
            .wrap_err("server responded with error code")?
            .json::<SubmitTransactionResponse>()
            .await
            .wrap_err("failed reading JSON response from server")?;
        Ok(response)
    }

    /// Submits the given transaction to the Sequencer node.
    /// This method blocks until the transaction is committed.
    pub async fn submit_transaction_commit(
        &self,
        tx: signed::Transaction,
    ) -> eyre::Result<SubmitTransactionCommitResponse> {
        let url = format!("{}/broadcast_tx_commit", self.base_url);
        let tx_bytes = tx.to_proto();
        let tx_hex = hex::encode(&tx_bytes);
        let params = [("tx", tx_hex)];
        let response = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .wrap_err("failed to send transaction")?;

        let response = response
            .error_for_status()
            .wrap_err("server responded with error code")?
            .json::<SubmitTransactionCommitResponse>()
            .await
            .wrap_err("failed reading JSON response from server")?;
        Ok(response)
    }
}

#[derive(Debug, Deserialize)]
pub struct SubmitTransactionResponse {
    pub code: String,
    pub data: String,
    pub log: String,
    pub codespace: String,
    // TODO: this is a hex string; decode it as such
    pub hash: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitTransactionCommitResponse {
    pub check_tx: TransactionResponse,
    pub deliver_tx: TransactionResponse,
    pub hash: String,
    pub height: String,
}

#[derive(Debug, Deserialize)]
pub struct TransactionResponse {
    pub code: String,
    pub data: String,
    pub log: String,
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub response: Response,
}

#[derive(Debug, Deserialize)]
pub struct Response {
    pub code: u32,
    pub log: String,
    pub index: u32,
    pub key: String,
    pub value: String,
    pub proof: String,
    pub height: u32,
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_get_balance() {
        let client = Client::default().unwrap();
        let address = Address::try_from("1c0c490f1b5528d8173c5de46d131160e4b2c0c3").unwrap();
        let balance = client.get_balance(&address, 0).await.unwrap();
        assert_eq!(balance, Balance::from(1000000));
    }
}
