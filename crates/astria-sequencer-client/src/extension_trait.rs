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

use std::{
    future,
    pin::Pin,
    sync::Arc,
};

pub use astria_core::{
    primitive::v1::Address,
    protocol::{
        account::v1alpha1::{
            BalanceResponse,
            NonceResponse,
        },
        transaction::v1alpha1::SignedTransaction,
    },
    sequencerblock::v1alpha1::{
        block::SequencerBlockError,
        SequencerBlock,
    },
};
use async_trait::async_trait;
use futures::Stream;
use prost::{
    DecodeError,
    Message as _,
};
use tendermint::block::Height;
#[cfg(feature = "http")]
use tendermint_rpc::HttpClient;
#[cfg(feature = "websocket")]
use tendermint_rpc::WebSocketClient;
use tendermint_rpc::{
    endpoint::broadcast::{
        tx_commit,
        tx_sync,
    },
    event::EventData,
    Client,
    SubscriptionClient,
};

#[cfg(feature = "http")]
impl SequencerClientExt for HttpClient {}
#[cfg(feature = "websocket")]
const _: () = {
    impl SequencerClientExt for WebSocketClient {}
    impl SequencerSubscriptionClientExt for WebSocketClient {}
};

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
            ErrorKind::CometBftConversion(e) => Some(e),
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

    #[must_use]
    pub fn as_tendermint_rpc(&self) -> Option<&TendermintRpcError> {
        match self.kind() {
            ErrorKind::TendermintRpc(e) => Some(e),
            _ => None,
        }
    }

    /// Convenience function to construct `Error` containing an `AbciQueryDeserializationError`.
    fn abci_query_deserialization(
        target: &'static str,
        response: tendermint_rpc::endpoint::abci_query::AbciQuery,
        inner: DecodeError,
    ) -> Self {
        Self {
            inner: ErrorKind::abci_query_deserialization(target, response, inner),
        }
    }

    fn cometbft_conversion(e: SequencerBlockError) -> Self {
        Self {
            inner: ErrorKind::CometBftConversion(e),
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
#[derive(Clone, Debug)]
pub struct AbciQueryDeserializationError {
    inner: DecodeError,
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
#[derive(Clone, Debug)]
pub struct TendermintRpcError {
    inner: tendermint_rpc::error::Error,
    rpc: &'static str,
}

impl TendermintRpcError {
    /// Utility to check if the underlying error is related to the transport failing.
    ///
    /// This is useful when trying to understand if a request failed because the underlying
    /// connection failed.
    #[must_use]
    pub fn is_transport(&self) -> bool {
        use tendermint_rpc::error::ErrorDetail;
        match &self.inner.detail() {
            // - ChannelSend is returned if the channel that WebSocketClient uses to communicate
            //   with the driver fails. This is the case if the driver has already failed, but the
            //   client still in use (there is no feedback mechanism between driver and its clients
            //   other than client commands failing).
            // - ClientInternal is returned by WebSocketClient if the channel the client sent to the
            //   websocket driver is dropped. This is the case if the driver recives the channel as
            //   part of a client's requests to the driver to send a message over the websocket, but
            //   then exits, dropping channel.
            ErrorDetail::ChannelSend(_) | ErrorDetail::ClientInternal(_) => true,
            _other => false,
        }
    }

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

#[derive(Clone, Debug)]
pub struct DeserializationError {
    inner: Arc<dyn std::error::Error + Send + Sync>,
    target: &'static str,
}

impl std::fmt::Display for DeserializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed deserializing cometbft response to {}",
            self.target,
        )
    }
}

impl std::error::Error for DeserializationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}

/// The collection of different errors that can occur when using the extension trait.
///
/// Note that none of the errors contained herein are constructable outside this crate.
#[derive(Debug)]
pub enum ErrorKind {
    AbciQueryDeserialization(AbciQueryDeserializationError),
    CometBftConversion(SequencerBlockError),
    TendermintRpc(TendermintRpcError),
}

impl ErrorKind {
    /// Convenience method to construct an `AbciQueryDeserialization` variant.
    fn abci_query_deserialization(
        target: &'static str,
        response: tendermint_rpc::endpoint::abci_query::AbciQuery,
        inner: DecodeError,
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

#[derive(Debug, thiserror::Error)]
pub enum NewBlockStreamError {
    #[error("failed converting new block received from CometBft to sequencer block")]
    CometBftConversion(#[source] SequencerBlockError),
    #[error("expected a `new-block` event, but got `{received}`")]
    UnexpectedEvent { received: &'static str },
    #[error("received a `new-block` event, but block field was not set")]
    NoBlock,
    #[error("encountered an error while receiving events over subscription")]
    Rpc(#[source] tendermint_rpc::Error),
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("failed subscribing to new cometbft blocks")]
pub struct SubscriptionFailed {
    #[from]
    source: tendermint_rpc::Error,
}

impl NewBlockStreamError {
    fn unexpected_event(event: &EventData) -> Self {
        fn event_to_name(event: &EventData) -> &'static str {
            match event {
                EventData::NewBlock {
                    ..
                } => "new-block",
                EventData::LegacyNewBlock {
                    ..
                } => "legacy-new-block",
                EventData::Tx {
                    ..
                } => "tx",
                EventData::GenericJsonEvent(_) => "generic-json",
            }
        }
        Self::UnexpectedEvent {
            received: event_to_name(event),
        }
    }
}

pub struct NewBlocksStream {
    inner: Pin<Box<dyn Stream<Item = Result<SequencerBlock, NewBlockStreamError>> + Send>>,
}

impl Stream for NewBlocksStream {
    type Item = Result<SequencerBlock, NewBlockStreamError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

pub struct LatestHeightStream {
    inner: Pin<Box<dyn Stream<Item = Result<Height, NewBlockStreamError>> + Send>>,
}

impl Stream for LatestHeightStream {
    type Item = Result<Height, NewBlockStreamError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[async_trait]
pub trait SequencerSubscriptionClientExt: SubscriptionClient {
    /// Subscribe to sequencer blocks from cometbft, returning a stream.
    ///
    /// This trait method calls the cometbft `/subscribe` endpoint with a
    /// `tm.event = 'NewBlock'` argument, and then attempts to convert each
    /// cometbft block to a [`SequencerBlock`] type.
    async fn subscribe_new_block_data(&self) -> Result<NewBlocksStream, SubscriptionFailed> {
        use futures::stream::{
            StreamExt as _,
            TryStreamExt as _,
        };
        use tendermint_rpc::query::{
            EventType,
            Query,
        };
        let stream = self
            .subscribe(Query::from(EventType::NewBlock))
            .await?
            .map_err(NewBlockStreamError::Rpc)
            .and_then(|event| {
                future::ready(match event.data {
                    EventData::LegacyNewBlock {
                        block: Some(block),
                        ..
                    } => SequencerBlock::try_from_cometbft(*block)
                        .map_err(NewBlockStreamError::CometBftConversion),

                    EventData::LegacyNewBlock {
                        block: None, ..
                    } => Err(NewBlockStreamError::NoBlock),

                    other => Err(NewBlockStreamError::unexpected_event(&other)),
                })
            })
            .boxed();
        Ok(NewBlocksStream {
            inner: stream,
        })
    }

    async fn subscribe_latest_height(&self) -> Result<LatestHeightStream, SubscriptionFailed> {
        use futures::stream::{
            StreamExt as _,
            TryStreamExt as _,
        };
        use tendermint_rpc::query::{
            EventType,
            Query,
        };
        let stream = self
            .subscribe(Query::from(EventType::NewBlock))
            .await?
            .map_err(NewBlockStreamError::Rpc)
            .and_then(|event| {
                future::ready(match event.data {
                    EventData::LegacyNewBlock {
                        block: Some(block),
                        ..
                    } => Ok(block.header.height),

                    EventData::LegacyNewBlock {
                        block: None, ..
                    } => Err(NewBlockStreamError::NoBlock),

                    other => Err(NewBlockStreamError::unexpected_event(&other)),
                })
            })
            .boxed();
        Ok(LatestHeightStream {
            inner: stream,
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
    ///   `astria.sequencer.v1.BalanceResponse`.
    async fn get_balance<AddressT, HeightT>(
        &self,
        address: AddressT,
        height: HeightT,
    ) -> Result<BalanceResponse, Error>
    where
        AddressT: Into<Address> + Send,
        HeightT: Into<tendermint::block::Height> + Send,
    {
        const PREFIX: &[u8] = b"accounts/balance/";

        let path = make_path_from_prefix_and_address(PREFIX, address.into().get());

        let response = self
            .abci_query(Some(path), vec![], Some(height.into()), false)
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        let proto_response =
            astria_core::generated::protocol::account::v1alpha1::BalanceResponse::decode(&*response.value)
                .map_err(|e| {
                Error::abci_query_deserialization(
                    "astria.sequencer.v1.BalanceResponse",
                    response,
                    e,
                )
            })?;
        Ok(proto_response.to_native())
    }

    /// Returns the current balance of the given account at the latest height.
    ///
    /// # Errors
    ///
    /// This has the same error conditions as [`SequencerClientExt::get_balance`].
    async fn get_latest_balance<A: Into<Address> + Send>(
        &self,
        address: A,
    ) -> Result<BalanceResponse, Error> {
        // This makes use of the fact that a height `None` and `Some(0)` are
        // treated the same.
        self.get_balance(address, 0u32).await
    }

    /// Returns the nonce of the given account at the given height.
    ///
    /// # Errors
    ///
    /// - If calling tendermint `abci_query` RPC fails.
    /// - If the bytes contained in the abci query response cannot be read as an
    ///   `astria.sequencer.v1.NonceResponse`.
    async fn get_nonce<AddressT, HeightT>(
        &self,
        address: AddressT,
        height: HeightT,
    ) -> Result<NonceResponse, Error>
    where
        AddressT: Into<Address> + Send,
        HeightT: Into<tendermint::block::Height> + Send,
    {
        const PREFIX: &[u8] = b"accounts/nonce/";

        let path = make_path_from_prefix_and_address(PREFIX, address.into().get());

        let response = self
            .abci_query(Some(path), vec![], Some(height.into()), false)
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        let proto_response =
            astria_core::generated::protocol::account::v1alpha1::NonceResponse::decode(&*response.value)
                .map_err(|e| {
                    Error::abci_query_deserialization(
                        "astria.sequencer.v1.NonceResponse",
                        response,
                        e,
                    )
                })?;
        Ok(proto_response.to_native())
    }

    /// Returns the current nonce of the given account at the latest height.
    ///
    /// # Errors
    ///
    /// This has the same error conditions as [`SequencerClientExt::get_nonce`].
    async fn get_latest_nonce<A: Into<Address> + Send>(
        &self,
        address: A,
    ) -> Result<NonceResponse, Error> {
        // This makes use of the fact that a height `None` and `Some(0)` are
        // treated the same.
        self.get_nonce(address, 0u32).await
    }

    /// Get the latest sequencer block.
    ///
    /// This is a convenience method that converts the result [`Client::latest_block`]
    /// to [`SequencerBlock`].
    async fn latest_sequencer_block(&self) -> Result<SequencerBlock, Error> {
        let rsp = self
            .latest_block()
            .await
            .map_err(|e| Error::tendermint_rpc("latest_block", e))?;
        SequencerBlock::try_from_cometbft(rsp.block).map_err(Error::cometbft_conversion)
    }

    /// Get the sequencer block at the provided height.
    ///
    /// This is a convenience method that converts the result of calling [`Client::block`]
    /// to an Astria [`SequencerBlock`].
    async fn sequencer_block<HeightT>(&self, height: HeightT) -> Result<SequencerBlock, Error>
    where
        HeightT: Into<tendermint::block::Height> + Send,
    {
        let rsp = self
            .block(height.into())
            .await
            .map_err(|e| Error::tendermint_rpc("block", e))?;
        SequencerBlock::try_from_cometbft(rsp.block).map_err(Error::cometbft_conversion)
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
        tx: SignedTransaction,
    ) -> Result<tx_sync::Response, Error> {
        let tx_bytes = tx.into_raw().encode_to_vec();
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
        tx: SignedTransaction,
    ) -> Result<tx_commit::Response, Error> {
        let tx_bytes = tx.into_raw().encode_to_vec();
        self.broadcast_tx_commit(tx_bytes)
            .await
            .map_err(|e| Error::tendermint_rpc("broadcast_tx_comit", e))
    }
}

pub(super) fn make_path_from_prefix_and_address(
    prefix: &'static [u8],
    address: [u8; 20],
) -> String {
    let mut path = vec![0u8; prefix.len() + address.len() * 2];
    path[..prefix.len()].copy_from_slice(prefix);
    hex::encode_to_slice(address, &mut path[prefix.len()..]).expect(
        "this is a bug: a buffer of sufficient size must have been allocated to hold 20 hex \
         encoded bytes",
    );
    String::from_utf8(path).expect("this is a bug: all bytes in the path buffer should be ascii")
}
