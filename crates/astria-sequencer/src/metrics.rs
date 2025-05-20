use std::time::Duration;

use telemetry::{
    metric_names,
    metrics::{
        Counter,
        Gauge,
        Histogram,
        RegisteringBuilder,
    },
};

const CHECK_TX_STAGE: &str = "stage";

pub struct Metrics {
    prepare_proposal_excluded_transactions_cometbft_space: Counter,
    prepare_proposal_excluded_transactions_sequencer_space: Counter,
    prepare_proposal_excluded_transactions_failed_execution: Counter,
    prepare_proposal_excluded_transactions: Gauge,
    proposal_deposits: Histogram,
    proposal_transactions: Histogram,
    process_proposal_skipped_proposal: Counter,
    check_tx_failed_tx_too_large: Counter,
    check_tx_failed_action_checks: Counter,
    check_tx_removed_expired: Counter,
    check_tx_removed_failed_execution: Counter,
    check_tx_duration_seconds_check_actions: Histogram,
    check_tx_duration_seconds_fetch_nonce: Histogram,
    check_tx_duration_seconds_recheck: Histogram,
    check_tx_duration_seconds_fetch_balances: Histogram,
    check_tx_duration_seconds_fetch_tx_cost: Histogram,
    check_tx_duration_seconds_insert_to_app_mempool: Histogram,
    check_tx_duration_seconds_transaction_status: Histogram,
    actions_per_transaction_in_mempool: Histogram,
    transaction_in_mempool_size_bytes: Histogram,
    transactions_in_mempool_total: Gauge,
    transactions_in_mempool_parked: Gauge,
    mempool_recosted: Counter,
    internal_logic_error: Counter,
    extended_commit_info_bytes: Histogram,
    extend_vote_duration_seconds: Histogram,
    extend_vote_failure_count: Counter,
    verify_vote_extension_failure_count: Counter,
}

impl Metrics {
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
        self.prepare_proposal_excluded_transactions.set(count);
    }

    pub(crate) fn record_proposal_deposits(&self, count: usize) {
        self.proposal_deposits.record(count);
    }

    pub(crate) fn record_proposal_transactions(&self, count: usize) {
        self.proposal_transactions.record(count);
    }

    pub(crate) fn increment_process_proposal_skipped_proposal(&self) {
        self.process_proposal_skipped_proposal.increment(1);
    }

    pub(crate) fn increment_check_tx_failed_tx_too_large(&self) {
        self.check_tx_failed_tx_too_large.increment(1);
    }

    pub(crate) fn increment_check_tx_failed_action_checks(&self) {
        self.check_tx_failed_action_checks.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_expired(&self) {
        self.check_tx_removed_expired.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_failed_execution(&self) {
        self.check_tx_removed_failed_execution.increment(1);
    }

    pub(crate) fn record_check_tx_duration_seconds_check_actions(&self, duration: Duration) {
        self.check_tx_duration_seconds_check_actions
            .record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_fetch_nonce(&self, duration: Duration) {
        self.check_tx_duration_seconds_fetch_nonce.record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_recheck(&self, duration: Duration) {
        self.check_tx_duration_seconds_recheck.record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_fetch_balances(&self, duration: Duration) {
        self.check_tx_duration_seconds_fetch_balances
            .record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_fetch_tx_cost(&self, duration: Duration) {
        self.check_tx_duration_seconds_fetch_tx_cost
            .record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_insert_to_app_mempool(
        &self,
        duration: Duration,
    ) {
        self.check_tx_duration_seconds_insert_to_app_mempool
            .record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_transaction_status(&self, duration: Duration) {
        self.check_tx_duration_seconds_transaction_status
            .record(duration);
    }

    pub(crate) fn record_actions_per_transaction_in_mempool(&self, count: usize) {
        self.actions_per_transaction_in_mempool.record(count);
    }

    pub(crate) fn record_transaction_in_mempool_size_bytes(&self, count: usize) {
        self.transaction_in_mempool_size_bytes.record(count);
    }

    pub(crate) fn set_transactions_in_mempool_total(&self, count: usize) {
        self.transactions_in_mempool_total.set(count);
    }

    pub(crate) fn set_transactions_in_mempool_parked(&self, count: usize) {
        self.transactions_in_mempool_parked.set(count);
    }

    pub(crate) fn increment_mempool_recosted(&self) {
        self.mempool_recosted.increment(1);
    }

    pub(crate) fn increment_internal_logic_error(&self) {
        self.internal_logic_error.increment(1);
    }

    pub(crate) fn record_extended_commit_info_bytes(&self, count: usize) {
        self.extended_commit_info_bytes.record(count);
    }

    pub(crate) fn record_extend_vote_duration_seconds(&self, duration: Duration) {
        self.extend_vote_duration_seconds.record(duration);
    }

    pub(crate) fn increment_extend_vote_failure_count(&self) {
        self.extend_vote_failure_count.increment(1);
    }

    pub(crate) fn increment_verify_vote_extension_failure_count(&self) {
        self.verify_vote_extension_failure_count.increment(1);
    }
}

impl telemetry::Metrics for Metrics {
    type Config = ();

    #[expect(
        clippy::too_many_lines,
        reason = "this is reasonable as we have a lot of metrics to register; the function is not \
                  complex, just long"
    )]
    fn register(
        builder: &mut RegisteringBuilder,
        _config: &Self::Config,
    ) -> Result<Self, telemetry::metrics::Error> {
        let prepare_proposal_excluded_transactions_cometbft_space = builder
            .new_counter_factory(
                PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE,
                "The number of transactions that have been excluded from blocks due to running \
                 out of space in the cometbft block",
            )?
            .register()?;

        let prepare_proposal_excluded_transactions_sequencer_space = builder
            .new_counter_factory(
                PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE,
                "The number of transactions that have been excluded from blocks due to running \
                 out of space in the sequencer block",
            )?
            .register()?;

        let prepare_proposal_excluded_transactions_failed_execution = builder
            .new_counter_factory(
                PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION,
                "The number of transactions that have been excluded from blocks due to failing to \
                 execute",
            )?
            .register()?;

        let prepare_proposal_excluded_transactions = builder
            .new_gauge_factory(
                PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS,
                "The number of excluded transactions in a proposal being prepared",
            )?
            .register()?;

        let proposal_deposits = builder
            .new_histogram_factory(PROPOSAL_DEPOSITS, "The number of deposits in a proposal")?
            .register()?;

        let proposal_transactions = builder
            .new_histogram_factory(
                PROPOSAL_TRANSACTIONS,
                "The number of transactions in a proposal",
            )?
            .register()?;

        let process_proposal_skipped_proposal = builder
            .new_counter_factory(
                PROCESS_PROPOSAL_SKIPPED_PROPOSAL,
                "The number of times our submitted prepared proposal was skipped in process \
                 proposal",
            )?
            .register()?;

        let check_tx_failed_tx_too_large = builder
            .new_counter_factory(
                CHECK_TX_FAILED_TX_TOO_LARGE,
                "The number of transactions that have failed checks due to being too large",
            )?
            .register()?;

        let check_tx_failed_action_checks = builder
            .new_counter_factory(
                CHECK_TX_FAILED_ACTION_CHECKS,
                "The number of transactions that have failed action checks",
            )?
            .register()?;

        let check_tx_removed_expired = builder
            .new_counter_factory(
                CHECK_TX_REMOVED_EXPIRED,
                "The number of transactions that have been removed from the mempool due to \
                 expiring in the app's mempool",
            )?
            .register()?;

        let check_tx_removed_failed_execution = builder
            .new_counter_factory(
                CHECK_TX_REMOVED_FAILED_EXECUTION,
                "The number of transactions that have been removed from the mempool due to \
                 failing execution in prepare_proposal",
            )?
            .register()?;

        let check_tx_duration_seconds_fetch_nonce = builder
            .new_histogram_factory(
                CHECK_TX_DURATION_SECONDS_FETCH_NONCE,
                "The amount of time taken in seconds to fetch an account's nonce",
            )?
            .register()?;

        let check_tx_duration_seconds_fetch_balances = builder
            .new_histogram_factory(
                CHECK_TX_DURATION_SECONDS_FETCH_BALANCES,
                "The amount of time taken in seconds to fetch balances",
            )?
            .register()?;

        let check_tx_duration_seconds_fetch_tx_cost = builder
            .new_histogram_factory(
                CHECK_TX_DURATION_SECONDS_FETCH_TX_COST,
                "The amount of time taken in seconds to fetch tx cost",
            )?
            .register()?;

        let check_tx_duration_seconds_transaction_status = builder
            .new_histogram_factory(
                CHECK_TX_DURATION_SECONDS_TRANSACTION_STATUS,
                "The amount of time taken in seconds to to check the transaction status in the \
                 app-side mempool",
            )?
            .register()?;

        let mut check_tx_duration_factory = builder.new_histogram_factory(
            CHECK_TX_DURATION_SECONDS,
            "The amount of time taken in seconds to successfully complete the various stages of \
             check_tx",
        )?;
        let check_tx_duration_seconds_check_actions = check_tx_duration_factory
            .register_with_labels(&[(CHECK_TX_STAGE, "actions check".to_string())])?;
        let check_tx_duration_seconds_recheck = check_tx_duration_factory
            .register_with_labels(&[(CHECK_TX_STAGE, "recheck".to_string())])?;
        let check_tx_duration_seconds_insert_to_app_mempool = check_tx_duration_factory
            .register_with_labels(&[(CHECK_TX_STAGE, "insert to app mempool".to_string())])?;

        let actions_per_transaction_in_mempool = builder
            .new_histogram_factory(
                ACTIONS_PER_TRANSACTION_IN_MEMPOOL,
                "The number of actions in a transaction added to the app mempool",
            )?
            .register()?;

        let transaction_in_mempool_size_bytes = builder
            .new_histogram_factory(
                TRANSACTION_IN_MEMPOOL_SIZE_BYTES,
                "The number of bytes in a transaction added to the app mempool",
            )?
            .register()?;

        let transactions_in_mempool_total = builder
            .new_gauge_factory(
                TRANSACTIONS_IN_MEMPOOL_TOTAL,
                "The number of transactions in the app mempool",
            )?
            .register()?;

        let transactions_in_mempool_parked = builder
            .new_gauge_factory(
                TRANSACTIONS_IN_MEMPOOL_PARKED,
                "The number of transactions parked in the app mempool",
            )?
            .register()?;

        let mempool_recosted = builder
            .new_counter_factory(
                MEMPOOL_RECOSTED,
                "The number of times the mempool has been recosted",
            )?
            .register()?;

        let internal_logic_error = builder
            .new_counter_factory(
                INTERNAL_LOGIC_ERROR,
                "The number of times a transaction has been rejected due to logic errors in the \
                 mempool",
            )?
            .register()?;

        let extended_commit_info_bytes = builder
            .new_histogram_factory(
                EXTENDED_COMMIT_INFO_BYTES,
                "The number of bytes in the extended commit info of the block",
            )?
            .register()?;

        let extend_vote_duration_seconds = builder
            .new_histogram_factory(
                EXTEND_VOTE_DURATION_SECONDS,
                "The amount of time taken in seconds to successfully create a vote extension",
            )?
            .register()?;

        let extend_vote_failure_count = builder
            .new_counter_factory(
                EXTEND_VOTE_FAILURE_COUNT,
                "The number of times the app has failed to extend the vote",
            )?
            .register()?;

        let verify_vote_extension_failure_count = builder
            .new_counter_factory(
                VERIFY_VOTE_EXTENSION_FAILURE_COUNT,
                "The number of times the app has failed to verify extended votes",
            )?
            .register()?;

        Ok(Self {
            prepare_proposal_excluded_transactions_cometbft_space,
            prepare_proposal_excluded_transactions_sequencer_space,
            prepare_proposal_excluded_transactions_failed_execution,
            prepare_proposal_excluded_transactions,
            proposal_deposits,
            proposal_transactions,
            process_proposal_skipped_proposal,
            check_tx_failed_tx_too_large,
            check_tx_failed_action_checks,
            check_tx_removed_expired,
            check_tx_removed_failed_execution,
            check_tx_duration_seconds_check_actions,
            check_tx_duration_seconds_fetch_nonce,
            check_tx_duration_seconds_recheck,
            check_tx_duration_seconds_fetch_balances,
            check_tx_duration_seconds_fetch_tx_cost,
            check_tx_duration_seconds_insert_to_app_mempool,
            check_tx_duration_seconds_transaction_status,
            actions_per_transaction_in_mempool,
            transaction_in_mempool_size_bytes,
            transactions_in_mempool_total,
            transactions_in_mempool_parked,
            mempool_recosted,
            internal_logic_error,
            extended_commit_info_bytes,
            extend_vote_duration_seconds,
            extend_vote_failure_count,
            verify_vote_extension_failure_count,
        })
    }
}

metric_names!(const METRICS_NAMES:
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_COMETBFT_SPACE,
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_SEQUENCER_SPACE,
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_FAILED_EXECUTION,
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS,
    PROPOSAL_DEPOSITS,
    PROPOSAL_TRANSACTIONS,
    PROCESS_PROPOSAL_SKIPPED_PROPOSAL,
    CHECK_TX_FAILED_TX_TOO_LARGE,
    CHECK_TX_FAILED_ACTION_CHECKS,
    CHECK_TX_REMOVED_EXPIRED,
    CHECK_TX_REMOVED_FAILED_EXECUTION,
    CHECK_TX_DURATION_SECONDS,
    CHECK_TX_DURATION_SECONDS_FETCH_NONCE,
    CHECK_TX_DURATION_SECONDS_FETCH_BALANCES,
    CHECK_TX_DURATION_SECONDS_TRANSACTION_STATUS,
    CHECK_TX_DURATION_SECONDS_FETCH_TX_COST,
    ACTIONS_PER_TRANSACTION_IN_MEMPOOL,
    TRANSACTION_IN_MEMPOOL_SIZE_BYTES,
    TRANSACTIONS_IN_MEMPOOL_TOTAL,
    TRANSACTIONS_IN_MEMPOOL_PARKED,
    MEMPOOL_RECOSTED,
    INTERNAL_LOGIC_ERROR,
    EXTENDED_COMMIT_INFO_BYTES,
    EXTEND_VOTE_DURATION_SECONDS,
    EXTEND_VOTE_FAILURE_COUNT,
    VERIFY_VOTE_EXTENSION_FAILURE_COUNT,
);

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_const(CHECK_TX_FAILED_TX_TOO_LARGE, "check_tx_failed_tx_too_large");
        assert_const(CHECK_TX_REMOVED_EXPIRED, "check_tx_removed_expired");
        assert_const(
            CHECK_TX_REMOVED_FAILED_EXECUTION,
            "check_tx_removed_failed_execution",
        );
        assert_const(
            CHECK_TX_FAILED_ACTION_CHECKS,
            "check_tx_failed_action_checks",
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
        assert_const(
            TRANSACTIONS_IN_MEMPOOL_PARKED,
            "transactions_in_mempool_parked",
        );
        assert_const(MEMPOOL_RECOSTED, "mempool_recosted");
        assert_const(INTERNAL_LOGIC_ERROR, "internal_logic_error");
        assert_const(EXTENDED_COMMIT_INFO_BYTES, "extended_commit_info_bytes");
        assert_const(EXTEND_VOTE_DURATION_SECONDS, "extend_vote_duration_seconds");
        assert_const(EXTEND_VOTE_FAILURE_COUNT, "extend_vote_failure_count");
        assert_const(
            VERIFY_VOTE_EXTENSION_FAILURE_COUNT,
            "verify_vote_extension_failure_count",
        );
    }
}
