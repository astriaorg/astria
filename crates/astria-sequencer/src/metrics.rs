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
use telemetry::metric_name;

pub(crate) struct Metrics {
    prepare_proposal_excluded_transactions_decode_failure: Counter,
    prepare_proposal_excluded_transactions_cometbft_space: Counter,
    prepare_proposal_excluded_transactions_sequencer_space: Counter,
    prepare_proposal_excluded_transactions_failed_execution: Counter,
    prepare_proposal_excluded_transactions: Gauge,
    proposal_deposits: Histogram,
    proposal_transactions: Histogram,
    process_proposal_skipped_proposal: Counter,
    check_tx_removed_too_large: Counter,
    check_tx_removed_failed_stateless: Counter,
    check_tx_removed_stale_nonce: Counter,
    check_tx_removed_account_balance: Counter,
}

impl Metrics {
    #[must_use]
    pub(crate) fn new() -> Self {
        describe_counter!(
            PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_DECODE_FAILURE,
            Unit::Count,
            "The number of transactions that have been excluded from blocks due to failing to \
             decode"
        );
        let prepare_proposal_excluded_transactions_decode_failure =
            counter!(PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_DECODE_FAILURE);

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
            CHECK_TX_REMOVED_FAILED_STATELESS,
            Unit::Count,
            "The number of transactions that have been removed from the mempool due to failing \
             the stateless check"
        );
        let check_tx_removed_failed_stateless = counter!(CHECK_TX_REMOVED_FAILED_STATELESS);

        describe_counter!(
            CHECK_TX_REMOVED_STALE_NONCE,
            Unit::Count,
            "The number of transactions that have been removed from the mempool due to having a \
             stale nonce"
        );
        let check_tx_removed_stale_nonce = counter!(CHECK_TX_REMOVED_STALE_NONCE);

        describe_counter!(
            CHECK_TX_REMOVED_ACCOUNT_BALANCE,
            Unit::Count,
            "The number of transactions that have been removed from the mempool due to having not \
             enough account balance"
        );
        let check_tx_removed_account_balance = counter!(CHECK_TX_REMOVED_ACCOUNT_BALANCE);

        Self {
            prepare_proposal_excluded_transactions_decode_failure,
            prepare_proposal_excluded_transactions_cometbft_space,
            prepare_proposal_excluded_transactions_sequencer_space,
            prepare_proposal_excluded_transactions_failed_execution,
            prepare_proposal_excluded_transactions,
            proposal_deposits,
            proposal_transactions,
            process_proposal_skipped_proposal,
            check_tx_removed_too_large,
            check_tx_removed_failed_stateless,
            check_tx_removed_stale_nonce,
            check_tx_removed_account_balance,
        }
    }

    pub(crate) fn increment_prepare_proposal_excluded_transactions_decode_failure(&self) {
        self.prepare_proposal_excluded_transactions_decode_failure
            .increment(1);
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

    pub(crate) fn set_prepare_proposal_excluded_transactions(&self, count: f64) {
        self.prepare_proposal_excluded_transactions.set(count);
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

    pub(crate) fn increment_check_tx_removed_too_large(&self) {
        self.check_tx_removed_too_large.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_failed_stateless(&self) {
        self.check_tx_removed_failed_stateless.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_stale_nonce(&self) {
        self.check_tx_removed_stale_nonce.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_account_balance(&self) {
        self.check_tx_removed_account_balance.increment(1);
    }
}

metric_name!(const PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_DECODE_FAILURE);
metric_name!(const PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE);
metric_name!(const PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE);
metric_name!(const PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION);
metric_name!(const PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS);
metric_name!(const PROPOSAL_DEPOSITS);
metric_name!(const PROPOSAL_TRANSACTIONS);
metric_name!(const PROCESS_PROPOSAL_SKIPPED_PROPOSAL);
metric_name!(const CHECK_TX_REMOVED_TOO_LARGE);
metric_name!(const CHECK_TX_REMOVED_FAILED_STATELESS);
metric_name!(const CHECK_TX_REMOVED_STALE_NONCE);
metric_name!(const CHECK_TX_REMOVED_ACCOUNT_BALANCE);

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
