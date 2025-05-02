use astria_core::protocol::transaction::v1::TransactionError;
use astria_eyre::eyre::Report;

use crate::mempool::{
    InsertionError,
    RemovalReason,
};

#[derive(Debug)]
pub(crate) enum CheckTxOutcome {
    AddedToPending,
    AddedToParked,
    AlreadyInPending,
    AlreadyInParked,
    FailedStatelessChecks {
        source: Report,
    },
    FailedInsertion(InsertionError),
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
