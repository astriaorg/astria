use astria_sequencer::{
    accounts::{
        query,
        types::{
            Address,
            Balance,
            Nonce,
        },
    },
    transaction::Signed,
};
use borsh::BorshDeserialize as _;
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
            tx_commit,
            tx_sync,
        },
        validators,
    },
    Client as _,
    HttpClient,
};

/// Default Tendermint base URL.
pub const DEFAULT_TENDERMINT_BASE_URL: &str = "http://localhost:26657";

/// Tendermint HTTP client which is used to interact with the Sequencer node.
#[derive(Clone)]
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

    pub fn inner(&self) -> &HttpClient {
        &self.client
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

    /// Returns the validator set at the given height.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    pub async fn get_validator_set<T: Into<Height>>(
        &self,
        height: T,
    ) -> eyre::Result<validators::Response> {
        self.client
            .validators(height.into(), tendermint_rpc::Paging::Default)
            .await
            .wrap_err("failed to get validator set")
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

        let maybe_balance =
            <query::Response as borsh::BorshDeserialize>::try_from_slice(&response.value)
                .wrap_err("failed to deserialize balance bytes")?;

        let query::Response::BalanceResponse(balance) = maybe_balance else {
            bail!(
                "received invalid response from server: {:?}, expected BalanceResponse, got \
                 variant: {:?}",
                &response,
                maybe_balance
            );
        };

        Ok(balance)
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
                Some(format!("accounts/nonce/{address}")),
                vec![],
                height,
                false,
            )
            .await
            .wrap_err("failed to call abci_query")?;

        let maybe_nonce = query::Response::try_from_slice(&response.value)
            .wrap_err("failed to deserialize balance bytes")?;

        let query::Response::NonceResponse(nonce) = maybe_nonce else {
            bail!(
                "received invalid response from server: {:?}, expected NonceResponse, got \
                 variant: {:?}",
                &response,
                maybe_nonce
            );
        };

        Ok(nonce)
    }

    /// Submits the given transaction to the Sequencer node.
    ///
    /// This method blocks until the transaction is checked, but not until it's committed.
    /// It returns the results of `CheckTx`.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    pub async fn submit_transaction_sync(&self, tx: Signed) -> eyre::Result<tx_sync::Response> {
        let tx_bytes = tx.to_bytes();
        self.client
            .broadcast_tx_sync(tx_bytes)
            .await
            .wrap_err("failed to call broadcast_tx_sync")
    }

    /// Submits the given transaction to the Sequencer node.
    ///
    /// This method blocks until the transaction is committed.
    /// It returns the results of `CheckTx` and `DeliverTx`.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    pub async fn submit_transaction_commit(&self, tx: Signed) -> eyre::Result<tx_commit::Response> {
        let tx_bytes = tx.to_bytes();
        self.client
            .broadcast_tx_commit(tx_bytes)
            .await
            .wrap_err("failed to call broadcast_tx_commit")
    }
}

#[cfg(test)]
mod test {
    use std::{
        str::FromStr,
        vec,
    };

    use astria_sequencer::{
        accounts::Transfer,
        transaction::{
            Action,
            Unsigned,
        },
    };
    use borsh::BorshSerialize;
    use ed25519_consensus::SigningKey;
    use serde_json::json;
    use tendermint::Hash;
    use tendermint_rpc::{
        endpoint::broadcast::tx_commit::DialectResponse,
        response::Wrapper,
        Id,
    };
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

        async fn register_abci_query_response(&self, query_path: &str, response: &query::Response) {
            let expected_body = json!({
                "method": "abci_query"
            });
            let response = tendermint_rpc::endpoint::abci_query::Response {
                response: tendermint_rpc::endpoint::abci_query::AbciQuery {
                    value: response.try_to_vec().unwrap(),
                    ..Default::default()
                },
            };
            let wrapper = Wrapper::new_with_id(Id::Num(1), Some(response), None);
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

        async fn register_broadcast_tx_sync_response(&self, response: tx_sync::Response) {
            let expected_body = json!({
                "method": "broadcast_tx_sync"
            });
            let wrapper = Wrapper::new_with_id(Id::Num(1), Some(response), None);
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
            response: DialectResponse<tendermint_rpc::dialect::v0_37::Event>,
        ) {
            let expected_body = json!({
                "method": "broadcast_tx_commit"
            });
            let wrapper = Wrapper::new_with_id(Id::Num(1), Some(response), None);
            Mock::given(body_partial_json(&expected_body))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(&wrapper)
                        .append_header("Content-Type", "application/json"),
                )
                .mount(&self.mock_server)
                .await;
        }

        async fn register_block_response(&self, response: BlockResponse) {
            let expected_body = json!({
                "method": "block"
            });
            let wrapper = Wrapper::new_with_id(Id::Num(1), Some(response), None);
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

        let actions = vec![Action::TransferAction(Transfer::new(
            Address::try_from_str(BOB_ADDRESS).unwrap(),
            Balance::from(333_333),
        ))];
        let tx = Unsigned::new_with_actions(Nonce::from(1), actions);
        tx.into_signed(&alice_keypair)
    }

    #[tokio::test]
    async fn get_nonce_and_balance() {
        let server = MockTendermintServer::new().await;

        let nonce_response = Nonce::from(0);
        server
            .register_abci_query_response(
                "accounts/nonce",
                &query::Response::NonceResponse(nonce_response),
            )
            .await;

        let balance_response = Balance::from(10_u128.pow(18));
        server
            .register_abci_query_response(
                "accounts/balance",
                &query::Response::BalanceResponse(balance_response),
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
    async fn submit_tx_sync() {
        let server = MockTendermintServer::new().await;

        let server_response = tx_sync::Response {
            code: 0.into(),
            data: vec![].into(),
            log: String::new(),
            hash: Hash::Sha256([0; 32]),
        };
        server
            .register_broadcast_tx_sync_response(server_response.clone())
            .await;
        let signed_tx = create_signed_transaction();

        let client = Client::new(&server.address()).unwrap();
        let response = client.submit_transaction_sync(signed_tx).await.unwrap();
        assert_eq!(response.code, server_response.code);
        assert_eq!(response.data, server_response.data);
        assert_eq!(response.log, server_response.log);
        assert_eq!(response.hash, server_response.hash);
    }

    #[tokio::test]
    async fn submit_tx_commit() {
        use tendermint_rpc::dialect;

        let server = MockTendermintServer::new().await;

        let server_response = DialectResponse::<tendermint_rpc::dialect::v0_37::Event> {
            check_tx: dialect::CheckTx::default(),
            deliver_tx: dialect::DeliverTx::default(),
            hash: Hash::Sha256([0; 32]),
            height: Height::from(1u32),
        };
        server
            .register_broadcast_tx_commit_response(server_response)
            .await;

        let signed_tx = create_signed_transaction();

        let client = Client::new(&server.address()).unwrap();
        let response = client.submit_transaction_commit(signed_tx).await.unwrap();
        assert_eq!(response.check_tx.code, 0.into());
        assert_eq!(response.deliver_tx.code, 0.into());
    }

    #[tokio::test]
    async fn get_latest_block() {
        use tendermint::{
            account,
            block::{
                header::Version,
                parts::Header as PartSetHeader,
                Block,
                Header,
                Height,
                Id as BlockId,
            },
            chain,
            evidence,
            hash::AppHash,
            Hash,
            Time,
        };

        let server = MockTendermintServer::new().await;

        let server_response = BlockResponse {
            block_id: BlockId {
                hash: Hash::Sha256([0; 32]),
                part_set_header: PartSetHeader::new(0, Hash::None).unwrap(),
            },
            block: Block::new(
                Header {
                    version: Version {
                        block: 0,
                        app: 0,
                    },
                    chain_id: chain::Id::try_from("test").unwrap(),
                    height: Height::from(1u32),
                    time: Time::now(),
                    last_block_id: None,
                    last_commit_hash: None,
                    data_hash: None,
                    validators_hash: Hash::Sha256([0; 32]),
                    next_validators_hash: Hash::Sha256([0; 32]),
                    consensus_hash: Hash::Sha256([0; 32]),
                    app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
                    last_results_hash: None,
                    evidence_hash: None,
                    proposer_address: account::Id::from_str(BOB_ADDRESS).unwrap(),
                },
                vec![],
                evidence::List::default(),
                None,
            )
            .unwrap(),
        };
        server.register_block_response(server_response).await;

        let client = Client::new(&server.address()).unwrap();
        let block = client.get_latest_block().await.unwrap();
        assert!(block.block.header.height.value() == 1);
    }
}
