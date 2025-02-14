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
//! use astria_core::primitive::v1::Address;
//! use astria_sequencer_client::SequencerClientExt as _;
//! use tendermint_rpc::HttpClient;
//!
//! let client = HttpClient::new("http://127.0.0.1:26657")?;
//! let address = Address::builder()
//!     .array(hex_literal::hex!(
//!         "DEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF"
//!     ))
//!     .prefix("astria")
//!     .try_build()
//!     .unwrap();
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
        account::v1::{
            BalanceResponse,
            NonceResponse,
        },
        transaction::v1::Transaction,
    },
    sequencerblock::v1::{
        block::SequencerBlockError,
        SequencerBlock,
    },
};
use astria_core::{
    protocol::{
        asset::v1::AllowedFeeAssetsResponse,
        bridge::v1::{
            BridgeAccountInfoResponse,
            BridgeAccountLastTxHashResponse,
        },
        fees::v1::TransactionFeeResponse,
        transaction::v1::{
            TransactionBody,
            TransactionStatus,
            TransactionStatusResponse,
        },
    },
    Protobuf as _,
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
    endpoint::broadcast::tx_sync,
    event::EventData,
    Client,
    SubscriptionClient,
};
use tracing::{
    debug,
    instrument,
    Level,
};

/// The maximum time to wait while receiving an `NOT_FOUND` transaction status from the sequencer
/// before erroring.
const MAX_TX_STATUS_NOT_FOUND_WAIT_TIME_SECS: u64 = 3;

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
            ErrorKind::TendermintRpc(e) => Some(e),
            ErrorKind::NativeConversion(e) => Some(e),
            ErrorKind::TxStatus(e) => Some(e),
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
            ErrorKind::AbciQueryDeserialization(_)
            | ErrorKind::NativeConversion(_)
            | ErrorKind::TxStatus(_) => None,
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

    /// Convenience function to construct `Error` containing a `TendermintRpcError`.
    fn tendermint_rpc(rpc: &'static str, inner: tendermint_rpc::error::Error) -> Self {
        Self {
            inner: ErrorKind::tendermint_rpc(rpc, inner),
        }
    }

    /// Convenience function to construct `Error` containing a `DeserializationError`.
    fn native_conversion(
        target: &'static str,
        inner: Arc<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        Self {
            inner: ErrorKind::native_conversion(target, inner),
        }
    }

    /// Convenience function to construct `Error` containing a `TxStatusError`.
    fn tx_status(inner: TxStatusError) -> Self {
        Self {
            inner: ErrorKind::TxStatus(inner),
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
            //   websocket driver is dropped. This is the case if the driver receives the channel as
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
            "failed deserializing raw protobuf response to {}",
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
    TendermintRpc(TendermintRpcError),
    NativeConversion(DeserializationError),
    TxStatus(TxStatusError),
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

    /// Convenience method to construct a `NativeConversion` variant.
    fn native_conversion(
        target: &'static str,
        inner: Arc<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        Self::NativeConversion(DeserializationError {
            inner,
            target,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct TxStatusError(TxStatusErrorKind);

#[derive(Debug, thiserror::Error)]
enum TxStatusErrorKind {
    #[error(
        "transaction status has remained not found for more than the maximum wait time: \
         {MAX_TX_STATUS_NOT_FOUND_WAIT_TIME_SECS} seconds"
    )]
    StatusNotFound,
    #[error(
        "the given transaction has been removed from the app mempool but has not been confirmed \
         in CometBFT: {0}"
    )]
    Removed(String),
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
    async fn get_balance<HeightT>(
        &self,
        address: Address,
        height: HeightT,
    ) -> Result<BalanceResponse, Error>
    where
        HeightT: Into<tendermint::block::Height> + Send,
    {
        const PREFIX: &str = "accounts/balance";
        let path = format!("{PREFIX}/{address}");

        let response = self
            .abci_query(Some(path), vec![], Some(height.into()), false)
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        let proto_response =
            astria_core::generated::astria::protocol::accounts::v1::BalanceResponse::decode(
                &*response.value,
            )
            .map_err(|e| {
                Error::abci_query_deserialization(
                    "astria.sequencer.v1.BalanceResponse",
                    response,
                    e,
                )
            })?;
        BalanceResponse::try_from_raw(&proto_response)
            .map_err(|e| Error::native_conversion("BalanceResponse", Arc::new(e)))
    }

    /// Returns the current balance of the given account at the latest height.
    ///
    /// # Errors
    ///
    /// This has the same error conditions as [`SequencerClientExt::get_balance`].
    async fn get_latest_balance(&self, address: Address) -> Result<BalanceResponse, Error> {
        // This makes use of the fact that a height `None` and `Some(0)` are
        // treated the same.
        self.get_balance(address, 0u32).await
    }

    /// Returns the allowed fee assets at a given height.
    ///
    /// # Errors
    ///
    /// - If calling tendermint `abci_query` RPC fails.
    /// - If the bytes contained in the abci query response cannot be deserialized as an
    ///  `astria.protocol.asset.v1.AllowedFeeAssetsResponse`.
    /// - If the raw response cannot be converted to the native type.
    async fn get_allowed_fee_assets(&self) -> Result<AllowedFeeAssetsResponse, Error> {
        let path = "asset/allowed_fee_assets".to_string();

        let response = self
            .abci_query(Some(path), vec![], Some(0u32.into()), false)
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        let proto_response =
            astria_core::generated::astria::protocol::asset::v1::AllowedFeeAssetsResponse::decode(
                &*response.value,
            )
            .map_err(|e| {
                Error::abci_query_deserialization(
                    "astria.protocol.asset.v1.AllowedFeeAssetsResponse",
                    response,
                    e,
                )
            })?;
        let native_response = AllowedFeeAssetsResponse::try_from_raw(&proto_response)
            .map_err(|e| Error::native_conversion("AllowedFeeAssetsResponse", Arc::new(e)))?;

        Ok(native_response)
    }

    /// Returns the nonce of the given account at the given height.
    ///
    /// # Errors
    ///
    /// - If calling tendermint `abci_query` RPC fails.
    /// - If the bytes contained in the abci query response cannot be read as an
    ///   `astria.sequencer.v1.NonceResponse`.
    async fn get_nonce<HeightT>(
        &self,
        address: Address,
        height: HeightT,
    ) -> Result<NonceResponse, Error>
    where
        HeightT: Into<tendermint::block::Height> + Send,
    {
        const PREFIX: &str = "accounts/nonce";
        let path = format!("{PREFIX}/{address}");

        let response = self
            .abci_query(Some(path), vec![], Some(height.into()), false)
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        let proto_response =
            astria_core::generated::astria::protocol::accounts::v1::NonceResponse::decode(
                &*response.value,
            )
            .map_err(|e| {
                Error::abci_query_deserialization("astria.sequencer.v1.NonceResponse", response, e)
            })?;
        Ok(proto_response.to_native())
    }

    /// Returns the current nonce of the given account at the latest height.
    ///
    /// # Errors
    ///
    /// This has the same error conditions as [`SequencerClientExt::get_nonce`].
    async fn get_latest_nonce(&self, address: Address) -> Result<NonceResponse, Error> {
        // This makes use of the fact that a height `None` and `Some(0)` are
        // treated the same.
        self.get_nonce(address, 0u32).await
    }

    async fn get_bridge_account_info(
        &self,
        address: Address,
    ) -> Result<BridgeAccountInfoResponse, Error> {
        const PREFIX: &str = "bridge/account_info";
        let path = format!("{PREFIX}/{address}");

        let response = self
            .abci_query(Some(path), vec![], None, false)
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        let proto_response =
            astria_core::generated::astria::protocol::bridge::v1::BridgeAccountInfoResponse::decode(
                &*response.value,
            )
            .map_err(|e| {
                Error::abci_query_deserialization(
                    "astria.protocol.bridge.v1.BridgeAccountInfoResponse",
                    response,
                    e,
                )
            })?;
        let native = BridgeAccountInfoResponse::try_from_raw(proto_response).map_err(|e| {
            Error::native_conversion(
                "astria.protocol.bridge.v1.BridgeAccountInfoResponse",
                Arc::new(e),
            )
        })?;
        Ok(native)
    }

    async fn get_bridge_account_last_transaction_hash(
        &self,
        address: Address,
    ) -> Result<BridgeAccountLastTxHashResponse, Error> {
        const PREFIX: &str = "bridge/account_last_tx_hash";
        let path = format!("{PREFIX}/{address}");

        let response = self
            .abci_query(Some(path), vec![], None, false)
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        let proto_response =
            astria_core::generated::astria::protocol::bridge::v1::BridgeAccountLastTxHashResponse::decode(
                &*response.value,
            )
            .map_err(|e| {
                Error::abci_query_deserialization(
                    "astria.protocol.bridge.v1.BridgeAccountLastTxHashResponse",
                    response,
                    e,
                )
            })?;
        let native = proto_response.try_into_native().map_err(|e| {
            Error::native_conversion(
                "astria.protocol.bridge.v1.BridgeAccountLastTxHashResponse",
                Arc::new(e),
            )
        })?;
        Ok(native)
    }

    async fn get_transaction_fee(
        &self,
        tx: TransactionBody,
    ) -> Result<TransactionFeeResponse, Error> {
        let path = "transaction/fee".to_string();
        let data = tx.into_raw().encode_to_vec();

        let response = self
            .abci_query(Some(path), data, None, false)
            .await
            .map_err(|e| Error::tendermint_rpc("abci_query", e))?;

        let proto_response =
            astria_core::generated::astria::protocol::fees::v1::TransactionFeeResponse::decode(
                &*response.value,
            )
            .map_err(|e| {
                Error::abci_query_deserialization(
                    "astria.protocol.transaction.v1.TransactionFeeResponse",
                    response,
                    e,
                )
            })?;
        let native = TransactionFeeResponse::try_from_raw(proto_response).map_err(|e| {
            Error::native_conversion(
                "astria.protocol.transaction.v1.TransactionFeeResponse",
                Arc::new(e),
            )
        })?;
        Ok(native)
    }

    /// Submits the given transaction to the Sequencer node.
    ///
    /// This method blocks until the transaction is checked, but not until it's committed.
    /// It returns the results of `CheckTx`.
    ///
    /// # Errors
    ///
    /// - If calling the tendermint RPC endpoint fails.
    async fn submit_transaction_sync(&self, tx: Transaction) -> Result<tx_sync::Response, Error> {
        let tx_bytes = tx.into_raw().encode_to_vec();
        self.broadcast_tx_sync(tx_bytes)
            .await
            .map_err(|e| Error::tendermint_rpc("broadcast_tx_sync", e))
    }

    /// Probes the sequencer for a transaction of given hash with a backoff.
    ///
    /// # Errors
    ///
    /// - If calling the ABCI query for transaction status fails or returns a bad response.
    /// - If the transaction is not found in the mempool within `MAX_WAIT_TIME_NOT_FOUND`.
    /// - If the transaction is removed from the mempool but not confirmed in CometBFT within
    ///   `MAX_WAIT_TIME_AFTER_REMOVAL`.
    #[instrument(skip_all, err(level = Level::INFO))]
    async fn confirm_tx_inclusion(
        &self,
        tx_hash: tendermint::hash::Hash,
    ) -> Result<tendermint_rpc::endpoint::tx::Response, Error> {
        use std::time::Duration;

        use astria_core::generated::astria::protocol::transaction::v1 as raw;
        use tokio::time::Instant;

        // The minimum milliseconds delay between receiving a transaction status response and
        // sending the next request.
        const MIN_POLL_INTERVAL_MILLIS: u64 = 100;
        // The maximum milliseconds delay between receiving a transaction status response and
        // sending the next request.
        const MAX_POLL_INTERVAL_MILLIS: u64 = 1000;
        // How long to wait after `confirm_tx_inclusion` is called before starting to log.
        const START_LOGGING_DELAY: Duration = Duration::from_millis(1000);
        // The duration between logging events. This is more than the maximum wait time for an
        // not found transaction status, but that is okay since a persistent not found status
        // will be logged when the error is returned.
        const LOG_INTERVAL: Duration = Duration::from_millis(5000);
        // The maximum time to wait for a transaction to show in the app mempool.
        const MAX_WAIT_TIME_NOT_FOUND: Duration =
            Duration::from_secs(MAX_TX_STATUS_NOT_FOUND_WAIT_TIME_SECS);
        // The maximum time to wait for a transaction to show as confirmed in CometBFT after being
        // removed from the app mempool.
        const MAX_RETRIES_AFTER_REMOVAL: u32 = 20;

        let start = Instant::now();
        let mut logged_at = start;

        let mut log_if_due = |status: &str| {
            if start.elapsed() <= START_LOGGING_DELAY || logged_at.elapsed() <= LOG_INTERVAL {
                return;
            }
            debug!(
                %status,
                %tx_hash,
                elapsed_seconds = start.elapsed().as_secs_f32(),
                "waiting to confirm transaction inclusion"
            );
            logged_at = Instant::now();
        };

        let mut sleep_millis = MIN_POLL_INTERVAL_MILLIS;

        // Polls sequencer for transaction status with a backoff. If the transaction is parked or
        // pending, this will continue polling until the transaction is removed from the mempool. If
        // the transaction status is not found, this means it has not yet made it to the mempool. We
        // give it `MAX_WAIT_TIME_NOT_FOUND` to change to a different status before erroring.
        let removal_reason = loop {
            tokio::time::sleep(Duration::from_millis(sleep_millis)).await;
            let rsp = self
                .abci_query(
                    Some(format!("transaction/status/{tx_hash}")),
                    vec![],
                    None,
                    false,
                )
                .await
                .map_err(|e| Error::tendermint_rpc("abci_query", e))?;
            let info = rsp.info.clone();
            let proto_response =
                raw::TransactionStatusResponse::decode(&*rsp.value).map_err(|e| {
                    Error::abci_query_deserialization(
                        "astria.protocol.transaction.v1.TransactionStatusResponse",
                        rsp,
                        e,
                    )
                })?;
            match TransactionStatusResponse::try_from_raw(proto_response)
                .map_err(|e| Error::native_conversion("TransactionStatusResponse", Arc::new(e)))?
                .status
            {
                TransactionStatus::NotFound => {
                    if start.elapsed() > MAX_WAIT_TIME_NOT_FOUND {
                        return Err(Error::tx_status(TxStatusError(
                            TxStatusErrorKind::StatusNotFound,
                        )));
                    }
                    log_if_due("NOT_FOUND");
                }
                TransactionStatus::Parked => {
                    log_if_due("PARKED");
                }
                TransactionStatus::Pending => {
                    log_if_due("PENDING");
                }
                TransactionStatus::RemovalCache => break info,
            };
            sleep_millis = (sleep_millis.saturating_mul(2)).min(MAX_POLL_INTERVAL_MILLIS);
        };

        // Note: using fixed backoff here. We expect the transaction to be confirmed quickly, this
        // is just to account for short delays or network issues.
        let retry_config = tryhard::RetryFutureConfig::new(MAX_RETRIES_AFTER_REMOVAL)
            .fixed_backoff(Duration::from_millis(100));
        tryhard::retry_fn(|| async {
            self.tx(tx_hash, false)
                .await
                .map_err(|err| {
                    debug!(%err, "failed to fetch transaction from CometBFT");
                })
                // check to ensure the result code is OK
                .and_then(|rsp| rsp.tx_result.code.is_ok().then_some(rsp).ok_or(()))
        })
        .with_config(retry_config)
        .await
        .map_err(|()| Error::tx_status(TxStatusError(TxStatusErrorKind::Removed(removal_reason))))
    }
}
