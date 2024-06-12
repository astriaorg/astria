//! Crate-specific metrics functionality.
//!
//! Registers metrics & lists constants to be used as metric names throughout crate.

use metrics::{
    describe_counter,
    describe_gauge,
    describe_histogram,
    Unit,
};
use telemetry::metric_names;

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

metric_names!(pub const METRICS_NAMES:
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_DECODE_FAILURE,
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE,
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE,
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION,
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS,
    PROPOSAL_DEPOSITS,
    PROPOSAL_TRANSACTIONS,
    PROCESS_PROPOSAL_SKIPPED_PROPOSAL,
    CHECK_TX_REMOVED_TOO_LARGE,
    CHECK_TX_REMOVED_FAILED_STATELESS,
    CHECK_TX_REMOVED_STALE_NONCE,
    CHECK_TX_REMOVED_ACCOUNT_BALANCE
);

#[cfg(test)]
mod tests {
    use super::{
        CHECK_TX_REMOVED_ACCOUNT_BALANCE,
        CHECK_TX_REMOVED_FAILED_STATELESS,
        CHECK_TX_REMOVED_STALE_NONCE,
        CHECK_TX_REMOVED_TOO_LARGE,
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS,
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE,
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_DECODE_FAILURE,
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION,
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE,
        PROCESS_PROPOSAL_SKIPPED_PROPOSAL,
        PROPOSAL_DEPOSITS,
        PROPOSAL_TRANSACTIONS,
    };

    #[track_caller]
    fn assert_const(actual: &'static str, suffix: &str) {
        // XXX: hard-code this so the crate name isn't accidentally changed.
        const CRATE_NAME: &str = "astria_sequencer";
        let expected = format!("{CRATE_NAME}_{suffix}");
        assert_eq!(expected, actual);
    }

    #[test]
    fn metrics_are_as_expected() {
        assert_const(
            PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_DECODE_FAILURE,
            "prepare_proposal_excluded_transactions_decode_failure",
        );
        assert_const(
            PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE,
            "prepare_proposal_excluded_transactions_cometbft_space",
        );
        assert_const(
            PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE,
            "prepare_proposal_excluded_transactions_sequencer_space",
        );
        assert_const(
            PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION,
            "prepare_proposal_excluded_transactions_failed_execution",
        );
        assert_const(
            PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS,
            "prepare_proposal_excluded_transactions",
        );
        assert_const(PROPOSAL_DEPOSITS, "proposal_deposits");
        assert_const(PROPOSAL_TRANSACTIONS, "proposal_transactions");
        assert_const(
            PROCESS_PROPOSAL_SKIPPED_PROPOSAL,
            "process_proposal_skipped_proposal",
        );
        assert_const(CHECK_TX_REMOVED_TOO_LARGE, "check_tx_removed_too_large");
        assert_const(
            CHECK_TX_REMOVED_FAILED_STATELESS,
            "check_tx_removed_failed_stateless",
        );
        assert_const(CHECK_TX_REMOVED_STALE_NONCE, "check_tx_removed_stale_nonce");
        assert_const(
            CHECK_TX_REMOVED_ACCOUNT_BALANCE,
            "check_tx_removed_account_balance",
        );
    }
}
