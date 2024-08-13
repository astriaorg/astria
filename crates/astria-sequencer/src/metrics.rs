use std::time::Duration;

use metrics::{
    counter,
    describe_counter,
    describe_gauge,
    describe_histogram,
    gauge,
    histogram,
    Counter,
    Gauge,
    Histogram,
    Unit,
};
use telemetry::metric_names;

const CHECK_TX_STAGE: &str = "stage";

pub(crate) struct Metrics {
    prepare_proposal_excluded_transactions_cometbft_space: Counter,
    prepare_proposal_excluded_transactions_sequencer_space: Counter,
    prepare_proposal_excluded_transactions_failed_execution: Counter,
    prepare_proposal_excluded_transactions: Gauge,
    proposal_deposits: Histogram,
    proposal_transactions: Histogram,
    process_proposal_skipped_proposal: Counter,
    check_tx_removed_failed_speculative_deliver_tx: Counter,
    check_tx_removed_too_large: Counter,
    check_tx_removed_expired: Counter,
    check_tx_removed_failed_deliver_tx: Counter,
    check_tx_duration_seconds_speculative_deliver_tx: Histogram,
    check_tx_duration_seconds_check_removed: Histogram,
    check_tx_duration_seconds_insert_to_app_mempool: Histogram,
    actions_per_transaction_in_mempool: Histogram,
    transaction_in_mempool_size_bytes: Histogram,
    transactions_in_mempool_total: Gauge,
}

impl Metrics {
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub(crate) fn new() -> Self {
        describe_counter!(
            PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE,
            Unit::Count,
            "The number of transactions that have been excluded from blocks due to running out of \
             space in the cometbft block"
        );
        let prepare_proposal_excluded_transactions_cometbft_space =
            counter!(PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE);

        describe_counter!(
            PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE,
            Unit::Count,
            "The number of transactions that have been excluded from blocks due to running out of \
             space in the sequencer block"
        );
        let prepare_proposal_excluded_transactions_sequencer_space =
            counter!(PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE);

        describe_counter!(
            PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION,
            Unit::Count,
            "The number of transactions that have been excluded from blocks due to failing to \
             execute"
        );
        let prepare_proposal_excluded_transactions_failed_execution =
            counter!(PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION);

        describe_gauge!(
            PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS,
            Unit::Count,
            "The number of excluded transactions in a proposal being prepared"
        );
        let prepare_proposal_excluded_transactions = gauge!(PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS);

        describe_histogram!(
            PROPOSAL_DEPOSITS,
            Unit::Count,
            "The number of deposits in a proposal"
        );
        let proposal_deposits = histogram!(PROPOSAL_DEPOSITS);

        describe_histogram!(
            PROPOSAL_TRANSACTIONS,
            Unit::Count,
            "The number of transactions in a proposal"
        );
        let proposal_transactions = histogram!(PROPOSAL_TRANSACTIONS);

        describe_counter!(
            PROCESS_PROPOSAL_SKIPPED_PROPOSAL,
            Unit::Count,
            "The number of times our submitted prepared proposal was skipped in process proposal"
        );
        let process_proposal_skipped_proposal = counter!(PROCESS_PROPOSAL_SKIPPED_PROPOSAL);

        describe_counter!(
            CHECK_TX_REMOVED_TOO_LARGE,
            Unit::Count,
            "The number of transactions that have been removed from the mempool due to being too \
             large"
        );
        let check_tx_removed_too_large = counter!(CHECK_TX_REMOVED_TOO_LARGE);

        describe_counter!(
            CHECK_TX_REMOVED_FAILED_SPECULATIVE_DELIVER_TX,
            Unit::Count,
            "The number of transactions that have been removed from the mempool due to failing
            a speculative deliver_tx"
        );
        let check_tx_removed_failed_speculative_deliver_tx =
            counter!(CHECK_TX_REMOVED_FAILED_SPECULATIVE_DELIVER_TX);

        describe_counter!(
            CHECK_TX_REMOVED_FAILED_DELIVER_TX,
            Unit::Count,
            "The number of transactions that have been removed from the mempool due to failing \
             deliver_tx in prepare_proposal()"
        );
        let check_tx_removed_failed_execution = counter!(CHECK_TX_REMOVED_FAILED_DELIVER_TX);

        describe_counter!(
            CHECK_TX_REMOVED_EXPIRED,
            Unit::Count,
            "The number of transactions that have been removed from the mempool due to expiring \
             in the app's mempool"
        );
        let check_tx_removed_expired = counter!(CHECK_TX_REMOVED_EXPIRED);

        describe_histogram!(
            CHECK_TX_DURATION_SECONDS,
            Unit::Seconds,
            "The amount of time taken in seconds to successfully complete the various stages of \
             check_tx"
        );
        let check_tx_duration_seconds_speculative_deliver_tx = histogram!(
            CHECK_TX_DURATION_SECONDS,
            CHECK_TX_STAGE => "speculative deliver tx to the app"
        );
        let check_tx_duration_seconds_check_removed = histogram!(
            CHECK_TX_DURATION_SECONDS,
            CHECK_TX_STAGE => "check for removal"
        );
        let check_tx_duration_seconds_insert_to_app_mempool = histogram!(
            CHECK_TX_DURATION_SECONDS,
            CHECK_TX_STAGE => "insert to app mempool"
        );

        describe_histogram!(
            ACTIONS_PER_TRANSACTION_IN_MEMPOOL,
            Unit::Count,
            "The number of actions in a transaction added to the app mempool"
        );
        let actions_per_transaction_in_mempool = histogram!(ACTIONS_PER_TRANSACTION_IN_MEMPOOL);

        describe_histogram!(
            TRANSACTION_IN_MEMPOOL_SIZE_BYTES,
            Unit::Bytes,
            "The number of bytes in a transaction added to the app mempool"
        );
        let transaction_in_mempool_size_bytes = histogram!(TRANSACTION_IN_MEMPOOL_SIZE_BYTES);

        describe_gauge!(
            TRANSACTIONS_IN_MEMPOOL_TOTAL,
            Unit::Count,
            "The number of transactions in the app mempool"
        );
        let transactions_in_mempool_total = gauge!(TRANSACTIONS_IN_MEMPOOL_TOTAL);

        Self {
            prepare_proposal_excluded_transactions_cometbft_space,
            prepare_proposal_excluded_transactions_sequencer_space,
            prepare_proposal_excluded_transactions_failed_execution,
            prepare_proposal_excluded_transactions,
            proposal_deposits,
            proposal_transactions,
            process_proposal_skipped_proposal,
            check_tx_removed_failed_speculative_deliver_tx,
            check_tx_removed_too_large,
            check_tx_removed_expired,
            check_tx_removed_failed_deliver_tx: check_tx_removed_failed_execution,
            check_tx_duration_seconds_speculative_deliver_tx,
            check_tx_duration_seconds_check_removed,
            check_tx_duration_seconds_insert_to_app_mempool,
            actions_per_transaction_in_mempool,
            transaction_in_mempool_size_bytes,
            transactions_in_mempool_total,
        }
    }

    pub(crate) fn increment_prepare_proposal_excluded_transactions_cometbft_space(&self) {
        self.prepare_proposal_excluded_transactions_cometbft_space
            .increment(1);
    }

    pub(crate) fn increment_prepare_proposal_excluded_transactions_sequencer_space(&self) {
        self.prepare_proposal_excluded_transactions_sequencer_space
            .increment(1);
    }

    pub(crate) fn increment_prepare_proposal_excluded_transactions_failed_execution(&self) {
        self.prepare_proposal_excluded_transactions_failed_execution
            .increment(1);
    }

    pub(crate) fn set_prepare_proposal_excluded_transactions(&self, count: usize) {
        #[allow(clippy::cast_precision_loss)]
        self.prepare_proposal_excluded_transactions
            .set(count as f64);
    }

    pub(crate) fn record_proposal_deposits(&self, count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.proposal_deposits.record(count as f64);
    }

    pub(crate) fn record_proposal_transactions(&self, count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.proposal_transactions.record(count as f64);
    }

    pub(crate) fn increment_process_proposal_skipped_proposal(&self) {
        self.process_proposal_skipped_proposal.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_failed_speculative_deliver_tx(&self) {
        self.check_tx_removed_failed_speculative_deliver_tx
            .increment(1);
    }

    pub(crate) fn increment_check_tx_removed_too_large(&self) {
        self.check_tx_removed_too_large.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_expired(&self) {
        self.check_tx_removed_expired.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_failed_execution(&self) {
        self.check_tx_removed_failed_deliver_tx.increment(1);
    }

    pub(crate) fn record_check_tx_duration_seconds_speculative_deliver_tx(
        &self,
        duration: Duration,
    ) {
        self.check_tx_duration_seconds_speculative_deliver_tx
            .record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_check_removed(&self, duration: Duration) {
        self.check_tx_duration_seconds_check_removed
            .record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_insert_to_app_mempool(
        &self,
        duration: Duration,
    ) {
        self.check_tx_duration_seconds_insert_to_app_mempool
            .record(duration);
    }

    pub(crate) fn record_actions_per_transaction_in_mempool(&self, count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.actions_per_transaction_in_mempool.record(count as f64);
    }

    pub(crate) fn record_transaction_in_mempool_size_bytes(&self, count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.transaction_in_mempool_size_bytes.record(count as f64);
    }

    pub(crate) fn set_transactions_in_mempool_total(&self, count: usize) {
        #[allow(clippy::cast_precision_loss)]
        self.transactions_in_mempool_total.set(count as f64);
    }
}

metric_names!(pub const METRICS_NAMES:
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE,
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE,
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION,
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS,
    PROPOSAL_DEPOSITS,
    PROPOSAL_TRANSACTIONS,
    PROCESS_PROPOSAL_SKIPPED_PROPOSAL,
    CHECK_TX_REMOVED_TOO_LARGE,
    CHECK_TX_REMOVED_EXPIRED,
    CHECK_TX_REMOVED_FAILED_SPECULATIVE_DELIVER_TX,
    CHECK_TX_REMOVED_FAILED_DELIVER_TX,
    CHECK_TX_REMOVED_FAILED_STATELESS,
    CHECK_TX_REMOVED_STALE_NONCE,
    CHECK_TX_REMOVED_ACCOUNT_BALANCE,
    CHECK_TX_DURATION_SECONDS,
    ACTIONS_PER_TRANSACTION_IN_MEMPOOL,
    TRANSACTION_IN_MEMPOOL_SIZE_BYTES,
    TRANSACTIONS_IN_MEMPOOL_TOTAL
);

#[cfg(test)]
mod tests {
    use super::{
        ACTIONS_PER_TRANSACTION_IN_MEMPOOL,
        CHECK_TX_DURATION_SECONDS,
        CHECK_TX_REMOVED_ACCOUNT_BALANCE,
        CHECK_TX_REMOVED_EXPIRED,
        CHECK_TX_REMOVED_FAILED_DELIVER_TX,
        CHECK_TX_REMOVED_FAILED_SPECULATIVE_DELIVER_TX,
        CHECK_TX_REMOVED_FAILED_STATELESS,
        CHECK_TX_REMOVED_STALE_NONCE,
        CHECK_TX_REMOVED_TOO_LARGE,
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS,
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE,
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION,
        PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE,
        PROCESS_PROPOSAL_SKIPPED_PROPOSAL,
        PROPOSAL_DEPOSITS,
        PROPOSAL_TRANSACTIONS,
        TRANSACTIONS_IN_MEMPOOL_TOTAL,
        TRANSACTION_IN_MEMPOOL_SIZE_BYTES,
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
        assert_const(
            CHECK_TX_REMOVED_FAILED_SPECULATIVE_DELIVER_TX,
            "check_tx_removed_failed_speculative_deliver_tx",
        );
        assert_const(CHECK_TX_REMOVED_TOO_LARGE, "check_tx_removed_too_large");
        assert_const(CHECK_TX_REMOVED_EXPIRED, "check_tx_removed_expired");
        assert_const(
            CHECK_TX_REMOVED_FAILED_DELIVER_TX,
            "check_tx_removed_failed_deliver_tx",
        );
        assert_const(
            CHECK_TX_REMOVED_FAILED_STATELESS,
            "check_tx_removed_failed_stateless",
        );
        assert_const(CHECK_TX_REMOVED_STALE_NONCE, "check_tx_removed_stale_nonce");
        assert_const(
            CHECK_TX_REMOVED_ACCOUNT_BALANCE,
            "check_tx_removed_account_balance",
        );
        assert_const(CHECK_TX_DURATION_SECONDS, "check_tx_duration_seconds");
        assert_const(
            ACTIONS_PER_TRANSACTION_IN_MEMPOOL,
            "actions_per_transaction_in_mempool",
        );
        assert_const(
            TRANSACTION_IN_MEMPOOL_SIZE_BYTES,
            "transaction_in_mempool_size_bytes",
        );
        assert_const(
            TRANSACTIONS_IN_MEMPOOL_TOTAL,
            "transactions_in_mempool_total",
        );
    }
}
