//! Crate-specific metrics functionality.
//!
//! Registers metrics & lists constants to be used as metric names throughout crate.

use metrics::{
    describe_counter,
    describe_gauge,
    describe_histogram,
    Unit,
};

/// Registers all metrics used by this crate.
pub fn register() {
    describe_counter!(
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_DECODE_FAILURE,
        Unit::Count,
        "The number of transactions that have been excluded from blocks due to failing to decode"
    );

    describe_counter!(
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE,
        Unit::Count,
        "The number of transactions that have been excluded from blocks due to running out of \
         space in the cometbft block"
    );

    describe_counter!(
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE,
        Unit::Count,
        "The number of transactions that have been excluded from blocks due to running out of \
         space in the sequencer block"
    );

    describe_counter!(
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION,
        Unit::Count,
        "The number of transactions that have been excluded from blocks due to failing to execute"
    );

    describe_gauge!(
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS,
        Unit::Count,
        "The number of excluded transactions in a proposal being prepared"
    );

    describe_gauge!(
        PROCESS_PROPOSAL_DEPOSIT_TRANSACTIONS,
        Unit::Count,
        "The number of deposit transactions in a proposal being processed"
    );

    describe_counter!(
        PROCESS_PROPOSAL_SKIPPED_PROPOSAL,
        Unit::Count,
        "The number of times our submitted prepared proposal was skipped in process proposal"
    );

    describe_counter!(
        CHECK_TX_REMOVED_TOO_LARGE,
        Unit::Count,
        "The number of transactions that have been removed from the mempool due to being too large"
    );

    describe_counter!(
        CHECK_TX_REMOVED_FAILED_TO_DECODE_PROTOBUF,
        Unit::Count,
        "The number of transactions that have been removed from the mempool due to failing to \
         decode from bytes into a protobuf"
    );

    describe_counter!(
        CHECK_TX_REMOVED_FAILED_TO_DECODE_SIGNED_TRANSACTION,
        Unit::Count,
        "The number of transactions that have been removed from the mempool due to failing to \
         decode from a protobuf into a valid signed transaction"
    );

    describe_counter!(
        CHECK_TX_REMOVED_FAILED_STATELESS,
        Unit::Count,
        "The number of transactions that have been removed from the mempool due to failing the \
         stateless check"
    );

    describe_counter!(
        CHECK_TX_REMOVED_STALE_NONCE,
        Unit::Count,
        "The number of transactions that have been removed from the mempool due to having a stale \
         nonce"
    );

    describe_counter!(
        CHECK_TX_REMOVED_ACCOUNT_BALANCE,
        Unit::Count,
        "The number of transactions that have been removed from the mempool due to having not \
         enough account balance"
    );

    describe_histogram!(
        PROCESS_PROPOSAL_TRANSACTIONS,
        Unit::Count,
        "The number of transactions in the process_proposal phase"
    );
}

// We configure buckets for manually, in order to ensure Prometheus metrics are structured as a
// Histogram, rather than as a Summary. These values are loosely based on guesses
// and may need to be updated over time.
pub const HISTOGRAM_BUCKETS: &[f64; 5] = &[25.0, 50.0, 100.0, 200.0, 1000.0];

pub const PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_DECODE_FAILURE: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_prepare_proposal_excluded_transactions_decode_failure"
);

pub const PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_prepare_proposal_excluded_transactions_cometbft_space"
);

pub const PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_prepare_proposal_excluded_transactions_sequencer_space"
);

pub const PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_prepare_proposal_excluded_transactions_failed_execution"
);

pub const PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_prepare_proposal_excluded_transactions"
);

pub const PROCESS_PROPOSAL_DEPOSIT_TRANSACTIONS: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_process_proposal_deposit_transactions"
);

pub const PROCESS_PROPOSAL_TRANSACTIONS: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_process_proposal_transactions");

pub const PROCESS_PROPOSAL_SKIPPED_PROPOSAL: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_process_proposal_skipped_proposal"
);

pub const CHECK_TX_REMOVED_TOO_LARGE: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_check_tx_removed_too_large");

pub const CHECK_TX_REMOVED_FAILED_TO_DECODE_PROTOBUF: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_check_tx_removed_failed_to_decode_protobuf"
);

pub const CHECK_TX_REMOVED_FAILED_TO_DECODE_SIGNED_TRANSACTION: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_check_tx_removed_failed_to_decode_signed_transaction"
);

pub const CHECK_TX_REMOVED_FAILED_STATELESS: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_check_tx_removed_failed_stateless"
);

pub const CHECK_TX_REMOVED_STALE_NONCE: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_check_tx_removed_stale_nonce");

pub const CHECK_TX_REMOVED_ACCOUNT_BALANCE: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_check_tx_removed_account_balance"
);
