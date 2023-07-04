use astria_sequencer::{
    accounts::{
        query::Response as QueryResponse,
        types::{
            Address,
            Balance,
            Nonce,
        },
    },
    transaction::Signed,
};
use borsh::BorshDeserialize;
use eyre::{
    self,
    bail,
    WrapErr as _,
};
use tendermint::block::Height;
use tendermint_rpc::{
    endpoint::{
        block::Response as BlockResponse,
        broadcast::{
            tx_commit::Response as BroadcastTxCommitResponse,
            tx_sync::Response as BroadcastTxSyncResponse,
        },
    },
    Client as _,
    HttpClient,
};

/// Default Tendermint base URL.
pub const DEFAULT_TENDERMINT_BASE_URL: &str = "http://localhost:26657";

/// Tendermint HTTP client which is used to interact with the Sequencer node.
pub struct Client {
    client: HttpClient,
}

impl Client {
    /// Creates a new Tendermint client with the given base URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the client could not be created.
    pub fn new(base_url: &str) -> eyre::Result<Self> {
        Ok(Client {
            client: HttpClient::new(base_url).wrap_err("failed to initialize tendermint client")?,
        })
    }

    /// Returns the latest block.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    pub async fn get_latest_block(&self) -> eyre::Result<BlockResponse> {
        let block = self
            .client
            .latest_block()
            .await
            .wrap_err("failed to call latest_block")?;
        Ok(block)
    }

    /// Returns the block at the given height.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    pub async fn get_block(&self, height: Height) -> eyre::Result<BlockResponse> {
        let block = self
            .client
            .block(height)
            .await
            .wrap_err("failed to call block")?;
        Ok(block)
    }

    /// Returns the balance of the given account at the given height.
    /// If no height is given, the latest height is used.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    /// - If the response from the server is invalid.
    pub async fn get_balance(
        &self,
        address: &Address,
        height: Option<Height>,
    ) -> eyre::Result<Balance> {
        let response = self
            .client
            .abci_query(
                Some(format!("accounts/balance/{}", &address.to_string())),
                vec![],
                height,
                false,
            )
            .await
            .wrap_err("failed to call abci_query")?;

        let balance = QueryResponse::try_from_slice(&response.value)
            .wrap_err("failed to deserialize balance bytes")?;

        if let QueryResponse::BalanceResponse(balance) = balance {
            Ok(balance)
        } else {
            bail!("received invalid response from server: {:?}", &response);
        }
    }

    /// Returns the nonce of the given account at the given height.
    /// If no height is given, the latest height is used.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    /// - If the response from the server is invalid.
    pub async fn get_nonce(
        &self,
        address: &Address,
        height: Option<Height>,
    ) -> eyre::Result<Nonce> {
        let response = self
            .client
            .abci_query(
                Some(format!("accounts/nonce/{}", &address.to_string())),
                vec![],
                height,
                false,
            )
            .await
            .wrap_err("failed to call abci_query")?;

        let nonce = QueryResponse::try_from_slice(&response.value)
            .wrap_err("failed to deserialize balance bytes")?;

        if let QueryResponse::NonceResponse(nonce) = nonce {
            Ok(nonce)
        } else {
            bail!("received invalid response from server: {:?}", &response);
        }
    }

    /// Submits the given transaction to the Sequencer node.
    /// This method blocks until the transaction is checked, but not until it's committed.
    /// It returns the results of `CheckTx`.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    pub async fn submit_transaction_sync(
        &self,
        tx: Signed,
    ) -> eyre::Result<BroadcastTxSyncResponse> {
        let tx_bytes = tx.to_bytes();
        self.client
            .broadcast_tx_sync(tx_bytes)
            .await
            .wrap_err("failed to call broadcast_tx_sync")
    }

    /// Submits the given transaction to the Sequencer node.
    /// This method blocks until the transaction is committed.
    /// It returns the results of `CheckTx` and `DeliverTx`.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    pub async fn submit_transaction_commit(
        &self,
        tx: Signed,
    ) -> eyre::Result<BroadcastTxCommitResponse> {
        let tx_bytes = tx.to_bytes();
        self.client
            .broadcast_tx_commit(tx_bytes)
            .await
            .wrap_err("failed to call broadcast_tx_commit")
    }
}

#[cfg(test)]
mod test {
    use astria_sequencer::{
        accounts::transaction::Transaction,
        transaction::Unsigned,
    };
    use borsh::BorshSerialize;
    use ed25519_consensus::SigningKey;
    use serde_json::json;
    use tendermint::Hash;
    use wiremock::{
        matchers::{
            body_partial_json,
            body_string_contains,
        },
        Mock,
        MockServer,
        ResponseTemplate,
    };

    use super::*;

    // see astria-sequencer/src/crypto.rs for how these keys/addresses were generated
    const ALICE_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
    const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";

    /// JSON-RPC response wrapper (i.e. message envelope)
    #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
    struct Wrapper<R> {
        /// JSON-RPC version
        jsonrpc: String,

        /// Identifier included in request
        id: u64,

        /// Results of request (if successful)
        result: Option<R>,

        /// Error message if unsuccessful
        error: Option<String>,
    }

    /// A mock tendermint server for testing.
    struct MockTendermintServer {
        mock_server: MockServer,
    }

    impl MockTendermintServer {
        async fn new() -> Self {
            let mock_server = MockServer::start().await;
            MockTendermintServer {
                mock_server,
            }
        }

        fn address(&self) -> String {
            format!("http://{}", self.mock_server.address())
        }

        async fn register_abci_query_response(&self, query_path: &str, response: &QueryResponse) {
            let expected_body = json!({
                "method": "abci_query"
            });
            let response = tendermint_rpc::endpoint::abci_query::Response {
                response: tendermint_rpc::endpoint::abci_query::AbciQuery {
                    value: response.try_to_vec().unwrap(),
                    ..Default::default()
                },
            };
            let wrapper = Wrapper {
                jsonrpc: "2.0".to_string(),
                id: 1,
                result: Some(response),
                error: None,
            };
            Mock::given(body_partial_json(&expected_body))
                .and(body_string_contains(query_path))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(&wrapper)
                        .append_header("Content-Type", "application/json"),
                )
                .mount(&self.mock_server)
                .await;
        }

        async fn register_broadcast_tx_sync_response(&self, response: &BroadcastTxSyncResponse) {
            let expected_body = json!({
                "method": "broadcast_tx_sync"
            });
            let wrapper = Wrapper {
                jsonrpc: "2.0".to_string(),
                id: 1,
                result: Some(response.clone()),
                error: None,
            };
            Mock::given(body_partial_json(&expected_body))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(&wrapper)
                        .append_header("Content-Type", "application/json"),
                )
                .mount(&self.mock_server)
                .await;
        }

        async fn register_broadcast_tx_commit_response(
            &self,
            response: &BroadcastTxCommitResponse,
        ) {
            let expected_body = json!({
                "method": "broadcast_tx_commit"
            });
            let wrapper = Wrapper {
                jsonrpc: "2.0".to_string(),
                id: 1,
                result: Some(response.clone()),
                error: None,
            };
            Mock::given(body_partial_json(&expected_body))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(&wrapper)
                        .append_header("Content-Type", "application/json"),
                )
                .mount(&self.mock_server)
                .await;
        }
    }

    fn create_signed_transaction() -> Signed {
        let alice_secret_bytes: [u8; 32] =
            hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
                .unwrap()
                .try_into()
                .unwrap();
        let alice_keypair = SigningKey::from(alice_secret_bytes);

        let tx = Unsigned::AccountsTransaction(Transaction::new(
            Address::try_from_str(BOB_ADDRESS).unwrap(),
            Balance::from(333_333),
            Nonce::from(1),
        ));
        tx.into_signed(&alice_keypair)
    }

    #[tokio::test]
    async fn test_get_nonce_and_balance() {
        let server = MockTendermintServer::new().await;

        let nonce_response = Nonce::from(0);
        server
            .register_abci_query_response(
                "accounts/nonce",
                &QueryResponse::NonceResponse(nonce_response),
            )
            .await;

        let balance_response = Balance::from(10_u128.pow(18));
        server
            .register_abci_query_response(
                "accounts/balance",
                &QueryResponse::BalanceResponse(balance_response),
            )
            .await;

        let client = Client::new(&server.address()).unwrap();
        let address = Address::try_from_str(ALICE_ADDRESS).unwrap();
        let nonce = client.get_nonce(&address, None).await.unwrap();
        assert_eq!(nonce, nonce_response);
        let balance = client.get_balance(&address, None).await.unwrap();
        assert_eq!(balance, balance_response);
    }

    #[tokio::test]
    async fn test_submit_tx_sync() {
        let server = MockTendermintServer::new().await;

        let server_response = BroadcastTxSyncResponse {
            code: 0.into(),
            data: vec![].into(),
            log: String::new(),
            hash: Hash::Sha256([0; 32]),
        };
        server
            .register_broadcast_tx_sync_response(&server_response)
            .await;
        let signed_tx = create_signed_transaction();

        let client = Client::new(&server.address()).unwrap();
        let response = client.submit_transaction_sync(signed_tx).await.unwrap();
        assert_eq!(response.code, server_response.code);
        assert_eq!(response.data, server_response.data);
        assert_eq!(response.log, server_response.log);
        assert_eq!(response.hash, server_response.hash);
    }

    #[ignore = "response parse error"]
    #[tokio::test]
    async fn test_submit_tx_commit() {
        let server = MockTendermintServer::new().await;

        let server_response = BroadcastTxCommitResponse {
            check_tx: tendermint::abci::response::CheckTx::default(),
            deliver_tx: tendermint::abci::response::DeliverTx::default(),
            hash: Hash::Sha256([0; 32]),
            height: Height::from(1u32),
        };
        server
            .register_broadcast_tx_commit_response(&server_response)
            .await;

        let signed_tx = create_signed_transaction();

        let client = Client::new(&server.address()).unwrap();
        let response = client.submit_transaction_commit(signed_tx).await.unwrap();
        assert_eq!(response.check_tx.code, 0.into());
        assert_eq!(response.deliver_tx.code, 0.into());
    }

    #[ignore = "requires running cometbft and sequencer node"]
    #[tokio::test]
    async fn test_get_latest_block() {
        let client = Client::new(DEFAULT_TENDERMINT_BASE_URL).unwrap();
        let block = client.get_latest_block().await.unwrap();
        assert!(block.block.header.height.value() >= 1);
    }
}
