use std::{
    fmt::{
        self,
        Display,
        Formatter,
    },
    num::ParseFloatError,
};

use prost::DecodeError;
use thiserror::Error;
use tonic::Status;

/// An error in sending or executing a gRPC via the `CelestiaClient`.
#[derive(Error, Clone, Debug)]
#[non_exhaustive]
pub(in crate::relayer) enum TrySubmitError {
    /// The celestia app responded with the given error status to a `QueryBlobParamsRequest`.
    #[error("failed to get blob params")]
    FailedToGetBlobParams(#[source] GrpcResponseError),
    /// The blob params response was empty.
    #[error("the blob params response was empty")]
    EmptyBlobParams,
    /// The celestia app responded with the given error status to a `QueryAuthParamsRequest`.
    #[error("failed to get auth params")]
    FailedToGetAuthParams(#[source] GrpcResponseError),
    /// The auth params response was empty.
    #[error("the auth params response was empty")]
    EmptyAuthParams,
    /// The celestia app responded with the given error status to a `MinGasPriceRequest`.
    #[error("failed to get minimum gas price")]
    FailedToGetMinGasPrice(#[source] GrpcResponseError),
    /// The minimum gas price response did not have the expected units suffix ("utia").
    #[error(
        "the minimum gas price response `{min_gas_price}` did not have the expected suffix \
         `{expected_suffix}`"
    )]
    MinGasPriceBadSuffix {
        min_gas_price: String,
        expected_suffix: &'static str,
    },
    /// The minimum gas price could not be parsed as a float.
    #[error("the minimum gas price `{min_gas_price}` could not be parsed as a float")]
    FailedToParseMinGasPrice {
        min_gas_price: String,
        source: ParseFloatError,
    },
    /// Blob size exceeds limit.
    #[error("blob size of {byte_count} bytes larger than limit of {}", u32::MAX)]
    BlobTooLarge { byte_count: usize },
    /// The celestia app responded with the given error status to a `QueryAccountRequest`.
    #[error("failed to get account info - is correct celestia signing key in use?")]
    FailedToGetAccountInfo(#[source] GrpcResponseError),
    /// The account info response was empty.
    #[error("the account info response was empty")]
    EmptyAccountInfo,
    /// The account info response was of an unexpected type.
    #[error("expected `{expected}` but received `{received}`")]
    AccountInfoTypeMismatch { expected: String, received: String },
    /// Failed to decode the received account info.
    #[error("failed to decode account info")]
    DecodeAccountInfo(#[source] ProtobufDecodeError),
    /// The celestia app responded with the given error status to a `BroadcastTxRequest`.
    #[error("failed to broadcast transaction")]
    FailedToBroadcastTx(#[source] GrpcResponseError),
    /// The broadcast transaction response was empty.
    #[error("the broadcast transaction response was empty")]
    EmptyBroadcastTxResponse,
    /// The broadcasted transaction response contains an error code.
    #[error(
        "broadcast transaction response contains error code `{code}`, tx `{tx_hash}`, namespace \
         `{namespace}`, log: `{log}`"
    )]
    BroadcastTxResponseErrorCode {
        tx_hash: String,
        code: u32,
        namespace: String,
        log: String,
    },
    /// The transaction was either evicted from the mempool or the call to `TxStatus` failed.
    #[error("failed to confirm transaction submission")]
    FailedToConfirmSubmission(#[source] ConfirmSubmissionError),
}

/// A gRPC status representing an error response from an RPC call.
#[derive(Clone, Debug)]
pub(in crate::relayer) struct GrpcResponseError(Status);

impl GrpcResponseError {
    pub(in crate::relayer) fn is_timeout(&self) -> bool {
        self.0.code() == tonic::Code::Cancelled
    }
}

impl Display for GrpcResponseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "status: {}, message: {}, metadata: {:?}",
            self.0.code(),
            self.0.message(),
            self.0.metadata(),
        )
    }
}

impl From<Status> for GrpcResponseError {
    fn from(status: Status) -> Self {
        Self(status)
    }
}

impl std::error::Error for GrpcResponseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// An error while decoding a Protobuf message.
#[derive(Error, Clone, Debug)]
#[error(transparent)]
pub(in crate::relayer) struct ProtobufDecodeError(#[from] DecodeError);

/// An error in getting the status of a transaction via RPC `TxStatus`.
#[derive(Debug, Clone, thiserror::Error)]
pub(in crate::relayer) enum TxStatusError {
    #[error("received unfamilair response for tx `{hash}` from `TxStatus`: {status}")]
    UnfamiliarStatus { status: String, hash: String },
    #[error("request for `TxStatus` failed: {error}")]
    // Using `String` here because jsonrpsee::core::Error does not implement `Clone`.
    FailedToGetTxStatus { error: String },
}

/// An error in confirming the submission of a transaction.
#[derive(Debug, Clone, thiserror::Error)]
pub(in crate::relayer) enum ConfirmSubmissionError {
    #[error("tx `{hash}` evicted from mempool")]
    Evicted { hash: String },
    #[error("received `UNKNOWN` status from `TxStatus` for tx: {hash}")]
    StatusUnknown { hash: String },
    #[error("failed to get tx status")]
    TxStatus(#[from] TxStatusError),
    #[error("received negative block height from Celestia: {height}")]
    NegativeHeight { height: i64 },
}
