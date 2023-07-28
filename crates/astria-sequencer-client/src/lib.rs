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
//! use astria_sequencer_client::{
//!     Address,
//!     Height,
//!     SequencerClientExt as _,
//! };
//! use tendermint_rpc::HttpClient;
//!
//! let client = HttpClient::new("http://127.0.0.1:26657")?;
//! let address = Address::try_from_str("DEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF")?;
//! let height = 5u32.into();
//! let balance = client.get_balance(&address, Some(height)).await?;
//! println!("{balance:?}");
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! # });
//! ```
use async_trait::async_trait;
pub use sequencer::accounts::types::{
    Address,
    Balance,
    Nonce,
};
// Reexports
pub use sequencer::transaction;
pub use tendermint::block::Height;
pub use tendermint_rpc::endpoint::broadcast::{
    tx_commit,
    tx_sync,
};
#[cfg(feature = "http")]
pub use tendermint_rpc::HttpClient;
#[cfg(feature = "websocket")]
pub use tendermint_rpc::WebSocketClient;

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
            ErrorKind::WrongQueryResponseKind(e) => Some(e),
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
        inner: borsh::maybestd::io::Error,
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

    /// Convenience function to construct `Error` containing a `WrongQueryResponseKind`.
    fn wrong_query_response_kind(
        expected: &'static str,
        received: sequencer::accounts::query::Response,
    ) -> Self {
        Self {
            inner: ErrorKind::wrong_query_response_kind(expected, received),
        }
    }
}

/// Error if deserialization of the bytes in an abci query response failed.
#[derive(Debug)]
pub struct AbciQueryDeserializationError {
    inner: borsh::maybestd::io::Error,
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

/// Error if the abci query response contained a sequencer response, but not the expected one.
#[derive(Debug)]
pub struct WrongQueryResponseKind {
    expected: &'static str,
    received: sequencer::accounts::query::Response,
}

impl WrongQueryResponseKind {
    /// Returns the name of the expected sequencer query response.
    #[must_use]
    pub fn expected(&self) -> &'static str {
        self.expected
    }

    /// Returns the actually received deserialized sequencer query response.
    #[must_use]
    pub fn received(&self) -> &sequencer::accounts::query::Response {
        &self.received
    }
}

impl std::fmt::Display for WrongQueryResponseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("did not receive expected sequencer response")
    }
}

impl std::error::Error for WrongQueryResponseKind {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

/// The collection of different errors that can occur when using the extension trait.
///
/// Note that none of the errors contained herein are constructable outside this crate.
#[derive(Debug)]
pub enum ErrorKind {
    AbciQueryDeserialization(AbciQueryDeserializationError),
    TendermintRpc(TendermintRpcError),
    WrongQueryResponseKind(WrongQueryResponseKind),
}

impl ErrorKind {
    /// Convenience method to construct an `AbciQueryDeserialization` variant.
    fn abci_query_deserialization(
        target: &'static str,
        response: tendermint_rpc::endpoint::abci_query::AbciQuery,
        inner: borsh::maybestd::io::Error,
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

    /// Convenience method to construct a `WrongQueryResponseKind` variant.
    fn wrong_query_response_kind(
        expected: &'static str,
        received: sequencer::accounts::query::Response,
    ) -> Self {
        Self::WrongQueryResponseKind(WrongQueryResponseKind {
            expected,
            received,
        })
    }
}

/// Tendermint HTTP client which is used to interact with the Sequencer node.
#[async_trait]
pub trait SequencerClientExt: tendermint_rpc::client::Client {
    /// Returns the balance of the given account at the given height.
    ///
    /// If `height = None`, the latest height is used.
    ///
    /// # Errors
    ///
    /// - If calling tendermint `abci_query` RPC fails.
    /// - If the opaque bytes cannot be deserialized as a sequencer query response.
    /// - If the deserialized sequencer query response is not a balance response.
    async fn get_balance(
        &self,
        address: &Address,
        height: Option<Height>,
    ) -> Result<Balance, Error> {
        use borsh::BorshDeserialize as _;
        use sequencer::accounts::query::Response;
        let height = height.map(Into::into);
        let response = self
            .abci_query(
                Some(format!("accounts/balance/{}", &address.to_string())),
                vec![],
                height,
                false,
            )
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        let maybe_balance = Response::try_from_slice(&response.value).map_err(|e| {
            Error::abci_query_deserialization("sequencer::accounts::query::Response", response, e)
        })?;

        // Allow this clippy lint because we want to always throw the other response into the error,
        // no matter if there is only one kind of response now or more in the future.
        #[allow(clippy::match_wildcard_for_single_variants)]
        match maybe_balance {
            Response::BalanceResponse(balance) => Ok(balance),
            other => Err(Error::wrong_query_response_kind("BalanceResponse", other)),
        }
    }

    /// Returns the nonce of the given account at the given height.
    ///
    /// If `height = None`, the latest height is used.
    ///
    /// # Errors
    ///
    /// - If calling tendermint `abci_query` RPC fails.
    /// - If the opaque bytes cannot be deserialized as a sequencer query response.
    /// - If the deserialized sequencer query response is not a balance response.
    async fn get_nonce(&self, address: &Address, height: Option<Height>) -> Result<Nonce, Error> {
        use borsh::BorshDeserialize as _;
        use sequencer::accounts::query::Response;

        let response = self
            .abci_query(
                Some(format!("accounts/nonce/{address}")),
                vec![],
                height,
                false,
            )
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        let maybe_nonce = Response::try_from_slice(&response.value).map_err(|e| {
            Error::abci_query_deserialization("sequencer::accounts::query::Response", response, e)
        })?;

        // Allow this clippy lint because we want to always throw the other response into the error,
        // no matter if there is only one kind of response now or more in the future.
        #[allow(clippy::match_wildcard_for_single_variants)]
        match maybe_nonce {
            Response::NonceResponse(nonce) => Ok(nonce),
            other => Err(Error::wrong_query_response_kind("NonceResponse", other)),
        }
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
