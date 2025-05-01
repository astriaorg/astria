use astria_core::protocol::transaction::v1::TransactionError;
use astria_eyre::eyre::Report;

use crate::mempool::{
    InsertionError,
    RemovalReason,
};

#[derive(Debug)]
pub(crate) enum CheckTxOutcome {
    AddedToParked,
    AddedToPending,
    AlreadyInParked,
    AlreadyInPending,
    FailedStatelessChecks {
        source: Report,
    },
    FailedInsertion(InsertionError),
    IncludedInBlock {
        height: u64,
    },
    InternalError {
        source: Report,
    },
    InvalidChainId {
        expected: String,
        actual: String,
    },
    InvalidTransactionProtobuf {
        source: TransactionError,
    },
    InvalidTransactionBytes {
        name: String,
        source: prost::DecodeError,
    },
    RemovedFromMempool(RemovalReason),
    TransactionTooLarge {
        max_size: usize,
        actual_size: usize,
    },
}

impl std::fmt::Display for CheckTxOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckTxOutcome::AddedToParked => write!(f, "transaction added to parked queue"),
            CheckTxOutcome::AddedToPending => write!(f, "transcation added to pending queue"),
            CheckTxOutcome::AlreadyInParked => {
                write!(f, "transaction already exists in parked queue")
            }
            CheckTxOutcome::AlreadyInPending => {
                write!(f, "transaction already exists in pending queue")
            }
            CheckTxOutcome::FailedStatelessChecks {
                source,
            } => write!(f, "failed stateless checks: {source}"),
            CheckTxOutcome::FailedInsertion(err) => write!(f, "failed insertion: {err}"),
            CheckTxOutcome::IncludedInBlock {
                height,
            } => write!(f, "included in block {height}"),
            CheckTxOutcome::InternalError {
                source,
            } => write!(f, "internal error: {source}"),
            CheckTxOutcome::InvalidChainId {
                expected,
                actual,
            } => {
                write!(f, "invalid chain id; expected: {expected}, got: {actual}")
            }
            CheckTxOutcome::InvalidTransactionProtobuf {
                source,
            } => {
                write!(f, "invalid transaction protobuf: {source}")
            }
            CheckTxOutcome::InvalidTransactionBytes {
                name,
                source,
            } => {
                write!(f, "failed decoding bytes as a protobuf {name}: {source}")
            }
            CheckTxOutcome::RemovedFromMempool(reason) => {
                write!(f, "removed from mempool: {reason}")
            }
            CheckTxOutcome::TransactionTooLarge {
                max_size,
                actual_size,
            } => {
                write!(
                    f,
                    "transaction size too large; allowed: {max_size} bytes, got {actual_size}"
                )
            }
        }
    }
}
