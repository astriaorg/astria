//! Extension trait for making tendermint JSONRPCs specific to astria-sequencer.
//!
//! [`SequencerClientExt`] is implemented for [`tendermint_rpc::HttpClient`] and
//! [`tendermint_rpc::WebSocketClient`], which are gated behind the features
//! `http` and `websocket`, respectively.
//!
//! # Examples
//! The example below works with the feature `"http"` set.
//! ```no_run
//! # tokio_test::block_on(async {
//! use astria_sequencer_client::SequencerClientExt as _;
//! use tendermint_rpc::HttpClient;
//!
//! let client = HttpClient::new("http://127.0.0.1:26657")?;
//! let address: [u8; 20] = hex_literal::hex!("DEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF");
//! let height = 5u32;
//! let balance = client.get_balance(address, height).await?;
//! println!("{balance:?}");
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! # });
//! ```
use async_trait::async_trait;
use proto::sequencer::v1alpha1::{
    BalanceResponse,
    NonceResponse,
};
// Reexports
pub use sequencer::transaction;
#[cfg(feature = "http")]
pub use tendermint_rpc::HttpClient;
#[cfg(feature = "websocket")]
pub use tendermint_rpc::WebSocketClient;
pub use tendermint_rpc::{
    client::Client,
    endpoint::broadcast::{
        tx_commit,
        tx_sync,
    },
};

#[cfg(feature = "http")]
impl SequencerClientExt for HttpClient {}
#[cfg(feature = "websocket")]
impl SequencerClientExt for WebSocketClient {}

#[cfg(test)]
mod tests;

/// An error that can occur when using one of the trait methods of `SequencerClientExt`.
///
/// The errors can be:
///
/// 1. the RPC call of the underlying tendermint client fails;
/// 2. the returned bytes contained in an `abci_query` RPC response cannot be deserialized as a
///    sequencer query response.
/// 3. the sequencer query response is not the expected one.
#[derive(Debug)]
pub struct Error {
    inner: ErrorKind,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("sequencer client method failed")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            ErrorKind::AbciQueryDeserialization(e) => Some(e),
            ErrorKind::TendermintRpc(e) => Some(e),
        }
    }
}

impl Error {
    /// Returns the reason why the request failed.
    #[must_use]
    pub fn kind(&self) -> &ErrorKind {
        &self.inner
    }

    /// Convenience function to construct `Error` containing an `AbciQueryDeserializationError`.
    fn abci_query_deserialization(
        target: &'static str,
        response: tendermint_rpc::endpoint::abci_query::AbciQuery,
        inner: proto::DecodeError,
    ) -> Self {
        Self {
            inner: ErrorKind::abci_query_deserialization(target, response, inner),
        }
    }

    /// Convenience function to construct `Error` containing a `TendermintRpcError`.
    fn tendermint_rpc(rpc: &'static str, inner: tendermint_rpc::error::Error) -> Self {
        Self {
            inner: ErrorKind::tendermint_rpc(rpc, inner),
        }
    }
}

/// Error if deserialization of the bytes in an abci query response failed.
#[derive(Debug)]
pub struct AbciQueryDeserializationError {
    inner: proto::DecodeError,
    response: Box<tendermint_rpc::endpoint::abci_query::AbciQuery>,
    target: &'static str,
}

impl AbciQueryDeserializationError {
    /// Returns the expected target type of the failed deserialization.
    #[must_use]
    pub fn target(&self) -> &'static str {
        self.target
    }

    /// Returns the original abci query response that could not be deserialized from.
    #[must_use]
    pub fn response(&self) -> &tendermint_rpc::endpoint::abci_query::AbciQuery {
        &self.response
    }
}

impl std::fmt::Display for AbciQueryDeserializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("failed deserializing bytes in ABCI query response")
    }
}

impl std::error::Error for AbciQueryDeserializationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}

/// Error if the rpc call using the underlying [`tendermint-rpc::client::Client`] failed.
#[derive(Debug)]
pub struct TendermintRpcError {
    inner: tendermint_rpc::error::Error,
    rpc: &'static str,
}

impl TendermintRpcError {
    /// Returns the error returned by the underlying tendermint RPC call.
    #[must_use]
    pub fn inner(&self) -> &tendermint_rpc::error::Error {
        &self.inner
    }

    /// Returns the name of the failed tendermint rpc call.
    #[must_use]
    pub fn rpc(&self) -> &'static str {
        self.rpc
    }
}

impl std::fmt::Display for TendermintRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("executing tendermint RPC failed")
    }
}

impl std::error::Error for TendermintRpcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

/// The collection of different errors that can occur when using the extension trait.
///
/// Note that none of the errors contained herein are constructable outside this crate.
#[derive(Debug)]
pub enum ErrorKind {
    AbciQueryDeserialization(AbciQueryDeserializationError),
    TendermintRpc(TendermintRpcError),
}

impl ErrorKind {
    /// Convenience method to construct an `AbciQueryDeserialization` variant.
    fn abci_query_deserialization(
        target: &'static str,
        response: tendermint_rpc::endpoint::abci_query::AbciQuery,
        inner: proto::DecodeError,
    ) -> Self {
        Self::AbciQueryDeserialization(AbciQueryDeserializationError {
            inner,
            response: Box::new(response),
            target,
        })
    }

    /// Convenience method to construct a `TendermintRpc` variant.
    fn tendermint_rpc(rpc: &'static str, inner: tendermint_rpc::error::Error) -> Self {
        Self::TendermintRpc(TendermintRpcError {
            inner,
            rpc,
        })
    }
}

/// Tendermint HTTP client which is used to interact with the Sequencer node.
#[async_trait]
pub trait SequencerClientExt: Client {
    /// Returns the balance of the given account at the given height.
    ///
    /// # Errors
    ///
    /// - If calling tendermint `abci_query` RPC fails.
    /// - If the bytes contained in the abci query response cannot be read as an
    ///   `astria.sequencer.v1alpha1.BalanceResponse`.
    async fn get_balance(&self, address: [u8; 20], height: u32) -> Result<BalanceResponse, Error> {
        use proto::Message as _;
        const PREFIX: &[u8] = b"accounts/balance/";

        let path = make_path_from_prefix_and_address(PREFIX, address);

        let response = self
            .abci_query(Some(path), vec![], Some(height.into()), false)
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        BalanceResponse::decode(&*response.value).map_err(|err| {
            Error::abci_query_deserialization(
                "astria.sequencer.v1alpha1.BalanceResponse",
                response,
                err,
            )
        })
    }

    /// Returns the current balance of the given account at the latest height.
    ///
    /// # Errors
    ///
    /// This has the same error conditions as [`SequencerClientExt::get_balance`].
    async fn get_latest_balance(&self, address: [u8; 20]) -> Result<BalanceResponse, Error> {
        // This makes use of the fact that a height `None` and `Some(0)` are
        // treated the same.
        self.get_balance(address, 0).await
    }

    /// Returns the nonce of the given account at the given height.
    ///
    /// If `height = None`, the latest height is used.
    ///
    /// # Errors
    ///
    /// - If calling tendermint `abci_query` RPC fails.
    /// - If the bytes contained in the abci query response cannot be read as an
    ///   `astria.sequencer.v1alpha1.NonceResponse`.
    async fn get_nonce(&self, address: [u8; 20], height: u32) -> Result<NonceResponse, Error> {
        use proto::Message as _;
        const PREFIX: &[u8] = b"accounts/nonce/";

        let path = make_path_from_prefix_and_address(PREFIX, address);

        let response = self
            .abci_query(Some(path), vec![], Some(height.into()), false)
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        NonceResponse::decode(&*response.value).map_err(|e| {
            Error::abci_query_deserialization(
                "astria::sequencer::v1alpha1.NonceResponse",
                response,
                e,
            )
        })
    }

    /// Returns the current nonce of the given account at the latest height.
    ///
    /// # Errors
    ///
    /// This has the same error conditions as [`SequencerClientExt::get_nonce`].
    async fn get_latest_nonce(&self, address: [u8; 20]) -> Result<NonceResponse, Error> {
        // This makes use of the fact that a height `None` and `Some(0)` are
        // treated the same.
        self.get_nonce(address, 0).await
    }

    /// Submits the given transaction to the Sequencer node.
    ///
    /// This method blocks until the transaction is checked, but not until it's committed.
    /// It returns the results of `CheckTx`.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    async fn submit_transaction_sync(
        &self,
        tx: transaction::Signed,
    ) -> Result<tx_sync::Response, Error> {
        let tx_bytes = tx.to_bytes();
        self.broadcast_tx_sync(tx_bytes)
            .await
            .map_err(|e| Error::tendermint_rpc("broadcast_tx_sync", e))
    }

    /// Submits the given transaction to the Sequencer node.
    ///
    /// This method blocks until the transaction is committed.
    /// It returns the results of `CheckTx` and `DeliverTx`.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    async fn submit_transaction_commit(
        &self,
        tx: transaction::Signed,
    ) -> Result<tx_commit::Response, Error> {
        let tx_bytes = tx.to_bytes();
        self.broadcast_tx_commit(tx_bytes)
            .await
            .map_err(|e| Error::tendermint_rpc("broadcast_tx_comit", e))
    }
}

fn make_path_from_prefix_and_address(prefix: &'static [u8], address: [u8; 20]) -> String {
    let mut path = vec![0u8; prefix.len() + address.len() * 2];
    path[..prefix.len()].copy_from_slice(prefix);
    hex::encode_to_slice(address, &mut path[prefix.len()..]).expect(
        "this is a bug: a buffer of sufficient size must have been allocated to hold 20 hex \
         encoded bytes",
    );
    String::from_utf8(path).expect("this is a bug: all bytes in the path buffer should be ascii")
}
