use astria_core::{
    generated::astria::protocol::transaction::v1 as raw,
    protocol::transaction::v1::TransactionError,
};
use astria_eyre::eyre;
use prost::{
    DecodeError,
    Name as _,
};
use thiserror::Error;

use super::MAX_TX_BYTES;
use crate::checked_actions::{
    CheckedActionExecutionError,
    CheckedActionInitialCheckError,
};

#[derive(Debug, Error)]
pub(crate) enum CheckedTransactionInitialCheckError {
    #[error("transaction size too large; allowed {MAX_TX_BYTES} bytes, got {tx_len} bytes")]
    TooLarge { max_len: usize, tx_len: usize },

    #[error(
        "failed decoding bytes as a protobuf `{}`: {0:#}",
        raw::Transaction::full_name()
    )]
    Decode(#[source] DecodeError),

    #[error(
        "failed converting protobuf `{}` to domain transaction: {0:#}",
        raw::Transaction::full_name()
    )]
    Convert(#[source] TransactionError),

    #[error(
        "transaction nonce already used; current nonce `{current_nonce}`, transaction nonce \
         `{tx_nonce}`"
    )]
    InvalidNonce { current_nonce: u32, tx_nonce: u32 },

    #[error(
        "transaction for wrong chain; expected chain id `{expected}`, transaction chain id \
         `{tx_chain_id}`"
    )]
    ChainIdMismatch {
        expected: String,
        tx_chain_id: String,
    },

    #[error(transparent)]
    CheckedAction(#[from] CheckedActionInitialCheckError),

    #[error("internal error: {context}: {source:#}")]
    InternalError {
        context: String,
        source: eyre::Report,
    },
}

impl CheckedTransactionInitialCheckError {
    pub(super) fn internal(context: &str, source: eyre::Report) -> Self {
        Self::InternalError {
            context: context.to_string(),
            source,
        }
    }
}

impl From<CheckedTransactionInitialCheckError> for tonic::Status {
    fn from(error: CheckedTransactionInitialCheckError) -> Self {
        let msg = error.to_string();
        match error {
            CheckedTransactionInitialCheckError::TooLarge {
                ..
            }
            | CheckedTransactionInitialCheckError::Decode(_)
            | CheckedTransactionInitialCheckError::Convert(_)
            | CheckedTransactionInitialCheckError::InvalidNonce {
                ..
            }
            | CheckedTransactionInitialCheckError::ChainIdMismatch {
                ..
            }
            | CheckedTransactionInitialCheckError::CheckedAction(_) => {
                tonic::Status::invalid_argument(msg)
            }
            CheckedTransactionInitialCheckError::InternalError {
                ..
            } => tonic::Status::internal(msg),
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum CheckedTransactionExecutionError {
    #[error(
        "invalid transaction nonce; expected nonce `{expected}`, transaction nonce `{tx_nonce}`"
    )]
    InvalidNonce { expected: u32, tx_nonce: u32 },

    #[error("overflow occurred incrementing stored nonce")]
    NonceOverflowed,

    #[error("overflow occurred incrementing action index")]
    ActionIndexOverflowed,

    #[error(transparent)]
    CheckedAction(#[from] CheckedActionExecutionError),

    #[error("internal error: {context}: {source:#}")]
    InternalError {
        context: String,
        source: eyre::Report,
    },
}

impl CheckedTransactionExecutionError {
    pub(super) fn internal(context: &str, source: eyre::Report) -> Self {
        Self::InternalError {
            context: context.to_string(),
            source,
        }
    }
}
