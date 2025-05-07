use astria_core::protocol::abci::AbciErrorCode;
use tendermint::abci::{
    response,
    Code,
};

use super::{
    error_response,
    outcome::CheckTxOutcome,
};
use crate::mempool::{
    InsertionError,
    RemovalReason,
};

pub(crate) trait IntoCheckTxResponse {
    fn into_check_tx_response(self) -> response::CheckTx;
}

impl IntoCheckTxResponse for RemovalReason {
    fn into_check_tx_response(self) -> response::CheckTx {
        match self {
            RemovalReason::Expired => response::CheckTx {
                code: Code::Err(AbciErrorCode::TRANSACTION_EXPIRED.value()),
                info: AbciErrorCode::TRANSACTION_EXPIRED.to_string(),
                log: "transaction expired in the app's mempool".into(),
                ..response::CheckTx::default()
            },
            RemovalReason::FailedPrepareProposal(err) => response::CheckTx {
                code: Code::Err(AbciErrorCode::TRANSACTION_FAILED.value()),
                info: AbciErrorCode::TRANSACTION_FAILED.to_string(),
                log: format!("transaction failed execution because: {err}"),
                ..response::CheckTx::default()
            },
            RemovalReason::NonceStale => response::CheckTx {
                code: Code::Err(AbciErrorCode::INVALID_NONCE.value()),
                info: "transaction removed from app mempool due to stale nonce".into(),
                log: "transaction from app mempool due to stale nonce".into(),
                ..response::CheckTx::default()
            },
            RemovalReason::LowerNonceInvalidated => response::CheckTx {
                code: Code::Err(AbciErrorCode::LOWER_NONCE_INVALIDATED.value()),
                info: AbciErrorCode::LOWER_NONCE_INVALIDATED.to_string(),
                log: "transaction removed from app mempool due to lower nonce being invalidated"
                    .into(),
                ..response::CheckTx::default()
            },
            RemovalReason::IncludedInBlock(height) => response::CheckTx {
                code: Code::Err(AbciErrorCode::TRANSACTION_INCLUDED_IN_BLOCK.value()),
                info: AbciErrorCode::TRANSACTION_INCLUDED_IN_BLOCK.to_string(),
                log: format!(
                    "transaction removed from app mempool because it was included in block \
                     {height}"
                ),
                ..response::CheckTx::default()
            },
            RemovalReason::InternalError => response::CheckTx {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: AbciErrorCode::INTERNAL_ERROR.to_string(),
                log: "transaction removed from app mempool due to internal error".to_string(),
                ..response::CheckTx::default()
            },
        }
    }
}

impl IntoCheckTxResponse for CheckTxOutcome {
    fn into_check_tx_response(self) -> response::CheckTx {
        match self {
            CheckTxOutcome::AddedToParked
            | CheckTxOutcome::AddedToPending
            | CheckTxOutcome::AlreadyInParked
            | CheckTxOutcome::AlreadyInPending => response::CheckTx::default(),
            CheckTxOutcome::FailedStatelessChecks {
                source,
            } => error_response(
                AbciErrorCode::TRANSACTION_FAILED_CHECK_TX,
                format!("transaction failed check tx: {source}"),
            ),
            CheckTxOutcome::FailedInsertion(err) => err.into_check_tx_response(),
            CheckTxOutcome::InternalError {
                source,
            } => error_response(
                AbciErrorCode::INTERNAL_ERROR,
                format!("internal error: {source}"),
            ),
            CheckTxOutcome::InvalidChainId {
                expected,
                actual,
            } => error_response(
                AbciErrorCode::INVALID_CHAIN_ID,
                format!("invalid chain id; expected: {expected}, got: {actual}"),
            ),
            CheckTxOutcome::InvalidTransactionProtobuf {
                source,
            } => error_response(
                AbciErrorCode::INVALID_TRANSACTION,
                format!("invalid transaction protobuf: {source}"),
            ),
            CheckTxOutcome::InvalidTransactionBytes {
                name,
                source,
            } => error_response(
                AbciErrorCode::INVALID_TRANSACTION_BYTES,
                format!("failed decoding bytes as a protobuf {name}: {source}"),
            ),
            CheckTxOutcome::RemovedFromMempool(removal_reason) => {
                removal_reason.into_check_tx_response()
            }
            CheckTxOutcome::TransactionTooLarge {
                max_size,
                actual_size,
            } => error_response(
                AbciErrorCode::TRANSACTION_TOO_LARGE,
                format!("transaction size too large; allowed: {max_size} bytes, got {actual_size}"),
            ),
        }
    }
}

impl IntoCheckTxResponse for InsertionError {
    fn into_check_tx_response(self) -> response::CheckTx {
        match self {
            InsertionError::AlreadyPresent => response::CheckTx {
                code: Code::Err(AbciErrorCode::ALREADY_PRESENT.value()),
                info: AbciErrorCode::ALREADY_PRESENT.to_string(),
                log: InsertionError::AlreadyPresent.to_string(),
                ..response::CheckTx::default()
            },
            InsertionError::NonceTooLow => response::CheckTx {
                code: Code::Err(AbciErrorCode::INVALID_NONCE.value()),
                info: AbciErrorCode::INVALID_NONCE.to_string(),
                log: InsertionError::NonceTooLow.to_string(),
                ..response::CheckTx::default()
            },
            InsertionError::NonceTaken => response::CheckTx {
                code: Code::Err(AbciErrorCode::NONCE_TAKEN.value()),
                info: AbciErrorCode::NONCE_TAKEN.to_string(),
                log: InsertionError::NonceTaken.to_string(),
                ..response::CheckTx::default()
            },
            InsertionError::AccountSizeLimit => response::CheckTx {
                code: Code::Err(AbciErrorCode::ACCOUNT_SIZE_LIMIT.value()),
                info: AbciErrorCode::ACCOUNT_SIZE_LIMIT.to_string(),
                log: InsertionError::AccountSizeLimit.to_string(),
                ..response::CheckTx::default()
            },
            InsertionError::ParkedSizeLimit => response::CheckTx {
                code: Code::Err(AbciErrorCode::PARKED_FULL.value()),
                info: AbciErrorCode::PARKED_FULL.info(),
                log: "transaction failed insertion because parked container is full".into(),
                ..response::CheckTx::default()
            },
            InsertionError::AccountBalanceTooLow | InsertionError::NonceGap => {
                // NOTE: these are handled interally by the mempool and don't
                // block transaction inclusion in the mempool. they shouldn't
                // be bubbled up to the client.
                response::CheckTx {
                    code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                    info: AbciErrorCode::INTERNAL_ERROR.info(),
                    log: "transaction failed insertion because of an internal error".into(),
                    ..response::CheckTx::default()
                }
            }
        }
    }
}
