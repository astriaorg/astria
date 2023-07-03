use astria_sequencer::{
    accounts::{
        query::Response as QueryResponse,
        types::{
            Address,
            Balance,
            Nonce,
        },
    },
    transaction::signed,
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
        tx: signed::Transaction,
    ) -> eyre::Result<BroadcastTxSyncResponse> {
        let tx_bytes = tx.to_proto();
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
        tx: signed::Transaction,
    ) -> eyre::Result<BroadcastTxCommitResponse> {
        let tx_bytes = tx.to_proto();
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
        transaction::unsigned::Transaction as UnsignedTransaction,
    };
    use ed25519_consensus::SigningKey;

    use super::*;

    const ALICE_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
    const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";

    #[ignore = "requires running cometbft and sequencer node"]
    #[tokio::test]
    async fn test_get_balance() {
        let client = Client::new(DEFAULT_TENDERMINT_BASE_URL).unwrap();
        let address = Address::try_from_str(ALICE_ADDRESS).unwrap();
        let nonce = client.get_nonce(&address, None).await.unwrap();
        assert_eq!(nonce, Nonce::from(0));
        let balance = client.get_balance(&address, None).await.unwrap();
        assert_eq!(balance, Balance::from(10_u128.pow(18)));
    }

    #[ignore = "requires running cometbft and sequencer node"]
    #[tokio::test]
    async fn test_submit_tx_commit() {
        let alice_secret_bytes: [u8; 32] =
            hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
                .unwrap()
                .try_into()
                .unwrap();
        let alice_keypair = SigningKey::from(alice_secret_bytes);

        let alice = Address::try_from_str(ALICE_ADDRESS).unwrap();
        let bob = Address::try_from_str(BOB_ADDRESS).unwrap();
        let value = Balance::from(333_333);
        let tx = UnsignedTransaction::AccountsTransaction(Transaction::new(
            bob.clone(),
            value,
            Nonce::from(1),
        ));
        let signed_tx = tx.sign(&alice_keypair);

        let client = Client::new(DEFAULT_TENDERMINT_BASE_URL).unwrap();
        let response = client.submit_transaction_commit(signed_tx).await.unwrap();
        assert_eq!(response.check_tx.code, 0.into());
        assert_eq!(response.deliver_tx.code, 0.into());
        let nonce = client.get_nonce(&alice, None).await.unwrap();
        assert_eq!(nonce, Nonce::from(1));
    }

    #[ignore = "requires running cometbft and sequencer node"]
    #[tokio::test]
    async fn test_get_latest_block() {
        let client = Client::new(DEFAULT_TENDERMINT_BASE_URL).unwrap();
        let block = client.get_latest_block().await.unwrap();
        assert!(block.block.header.height.value() >= 1);
    }
}
