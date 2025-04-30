use astria_core::protocol::{
    abci::AbciErrorCode,
    transaction::v1::TransactionError,
};
use astria_eyre::eyre::Report;
use tendermint::abci::response;

use super::{
    error_response,
    IntoCheckTxResponse as _,
};
use crate::mempool::{
    InsertionError,
    RemovalReason,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum CheckTxError {
    #[error("transaction failed stateless checks")]
    FailedStatelessChecks { source: Report },
    #[error("transaction failed insertion into the mempool: {0}")]
    FailedInsertion(InsertionError),
    #[error("transaction has already been included in block {block_number}")]
    Included { block_number: u64 },
    #[error("failed due to internal error")]
    InternalError { source: Report },
    #[error("transaction chain id mismatch; expected: {expected}, got: {actual}")]
    InvalidChainId { expected: String, actual: String },
    #[error("the provided transaction could not be validated")]
    InvalidTransactionProtobuf { source: TransactionError },
    #[error("failed decoding bytes as a protobuf {name}")]
    InvalidTransactionBytes {
        name: String,
        source: prost::DecodeError,
    },
    #[error("transaction has been removed from the app's mempool due to {0}")]
    RemovedFromMempool(RemovalReason),
    #[error("transaction is already tracked in the app's mempool")]
    Tracked,
    #[error("transaction size too large; allowed: {max_size} bytes, got {actual_size}")]
    TransactionTooLarge { max_size: usize, actual_size: usize },
}

impl From<CheckTxError> for response::CheckTx {
    fn from(err: CheckTxError) -> Self {
        match err {
            CheckTxError::FailedStatelessChecks {
                source,
            } => error_response(
                AbciErrorCode::FAILED_STATELESS_CHECKS,
                format!("transaction failed stateless checks: {source}"),
            ),
            CheckTxError::FailedInsertion(err) => err.into_check_tx_response(),
            CheckTxError::Included {
                block_number,
            } => error_response(
                AbciErrorCode::TRANSACTION_PREVIOUSLY_INCLUDED,
                format!("transaction has already been included in block {block_number}"),
            ),
            CheckTxError::InternalError {
                source,
            } => error_response(
                AbciErrorCode::INTERNAL_ERROR,
                format!("internal error: {source}"),
            ),
            CheckTxError::InvalidChainId {
                expected,
                actual,
            } => error_response(
                AbciErrorCode::INVALID_CHAIN_ID,
                format!("invalid chain id; expected: {expected}, got: {actual}"),
            ),
            CheckTxError::InvalidTransactionProtobuf {
                source,
            } => error_response(
                AbciErrorCode::INVALID_TRANSACTION,
                format!("invalid transaction protobuf: {source}"),
            ),
            CheckTxError::InvalidTransactionBytes {
                name,
                source,
            } => error_response(
                AbciErrorCode::INVALID_TRANSACTION_BYTES,
                format!("failed decoding bytes as a protobuf {name}: {source}"),
            ),
            CheckTxError::RemovedFromMempool(removal_reason) => {
                removal_reason.into_check_tx_response()
            }
            CheckTxError::Tracked => error_response(
                AbciErrorCode::ALREADY_PRESENT,
                "transaction is already tracked in the app's mempool".into(),
            ),
            CheckTxError::TransactionTooLarge {
                max_size,
                actual_size,
            } => error_response(
                AbciErrorCode::TRANSACTION_TOO_LARGE,
                format!("transaction size too large; allowed: {max_size} bytes, got {actual_size}"),
            ),
        }
    }
}

impl From<CheckTxError> for tonic::Status {
    fn from(err: CheckTxError) -> Self {
        match err {
            CheckTxError::FailedStatelessChecks {
                source,
            } => tonic::Status::internal(format!("transaction failed stateless checks: {source}")),
            CheckTxError::FailedInsertion(err) => err.into(),
            CheckTxError::Included {
                block_number,
            } => tonic::Status::failed_precondition(format!(
                "transaction has already been included in block {block_number}"
            )),
            CheckTxError::InternalError {
                source,
            } => tonic::Status::internal(format!("internal error: {source}")),
            CheckTxError::InvalidChainId {
                expected,
                actual,
            } => tonic::Status::invalid_argument(format!(
                "invalid chain id; expected: {expected}, got: {actual}"
            )),
            CheckTxError::InvalidTransactionProtobuf {
                source,
            } => tonic::Status::invalid_argument(format!("invalid transaction protobuf: {source}")),
            CheckTxError::InvalidTransactionBytes {
                name,
                source,
            } => tonic::Status::invalid_argument(format!(
                "failed decoding bytes as a protobuf {name}: {source}"
            )),
            CheckTxError::RemovedFromMempool(removal_reason) => tonic::Status::resource_exhausted(
                format!("transaction has been removed from the app-side mempool: {removal_reason}"),
            ),
            CheckTxError::Tracked => tonic::Status::already_exists(
                "transaction is already tracked in the app-side mempool".to_string(),
            ),
            CheckTxError::TransactionTooLarge {
                max_size,
                actual_size,
            } => tonic::Status::invalid_argument(format!(
                "transaction size too large; allowed: {max_size} bytes, got {actual_size}"
            )),
        }
    }
}
