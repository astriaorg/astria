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
