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
        PROPOSAL_TRANSACTIONS,
        Unit::Count,
        "The number of transactions in a proposal"
    );

    describe_histogram!(
        PROPOSAL_DEPOSITS,
        Unit::Count,
        "The number of deposits in a proposal"
    );
}

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

pub const PROPOSAL_DEPOSITS: &str = concat!(env!("CARGO_CRATE_NAME"), "_proposal_deposits");

pub const PROPOSAL_TRANSACTIONS: &str = concat!(env!("CARGO_CRATE_NAME"), "_proposal_transactions");

pub const PROCESS_PROPOSAL_SKIPPED_PROPOSAL: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_process_proposal_skipped_proposal"
);

pub const CHECK_TX_REMOVED_TOO_LARGE: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_check_tx_removed_too_large");

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
