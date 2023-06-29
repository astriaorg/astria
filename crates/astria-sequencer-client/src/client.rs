use std::time::Duration;

use astria_sequencer::transaction::signed;
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
