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
    check_tx_removed_too_large: Counter,
    check_tx_removed_expired: Counter,
    check_tx_removed_failed_execution: Counter,
    check_tx_removed_failed_stateless: Counter,
    check_tx_removed_stale_nonce: Counter,
    check_tx_duration_seconds_parse_tx: Histogram,
    check_tx_duration_seconds_check_stateless: Histogram,
    check_tx_duration_seconds_check_nonce: Histogram,
    check_tx_duration_seconds_check_chain_id: Histogram,
    check_tx_duration_seconds_check_removed: Histogram,
    check_tx_duration_seconds_convert_address: Histogram,
    check_tx_duration_seconds_fetch_balances: Histogram,
    check_tx_duration_seconds_fetch_tx_cost: Histogram,
    check_tx_duration_seconds_insert_to_app_mempool: Histogram,
    actions_per_transaction_in_mempool: Histogram,
    transaction_in_mempool_size_bytes: Histogram,
    transactions_in_mempool_total: Gauge,
    mempool_recosted: Counter,
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

    pub(crate) fn increment_check_tx_removed_too_large(&self) {
        self.check_tx_removed_too_large.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_expired(&self) {
        self.check_tx_removed_expired.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_failed_execution(&self) {
        self.check_tx_removed_failed_execution.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_failed_stateless(&self) {
        self.check_tx_removed_failed_stateless.increment(1);
    }

    pub(crate) fn increment_check_tx_removed_stale_nonce(&self) {
        self.check_tx_removed_stale_nonce.increment(1);
    }

    pub(crate) fn record_check_tx_duration_seconds_parse_tx(&self, duration: Duration) {
        self.check_tx_duration_seconds_parse_tx.record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_check_stateless(&self, duration: Duration) {
        self.check_tx_duration_seconds_check_stateless
            .record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_check_nonce(&self, duration: Duration) {
        self.check_tx_duration_seconds_check_nonce.record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_check_chain_id(&self, duration: Duration) {
        self.check_tx_duration_seconds_check_chain_id
            .record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_check_removed(&self, duration: Duration) {
        self.check_tx_duration_seconds_check_removed
            .record(duration);
    }

    pub(crate) fn record_check_tx_duration_seconds_convert_address(&self, duration: Duration) {
        self.check_tx_duration_seconds_convert_address
            .record(duration);
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

    pub(crate) fn record_actions_per_transaction_in_mempool(&self, count: usize) {
        self.actions_per_transaction_in_mempool.record(count);
    }

    pub(crate) fn record_transaction_in_mempool_size_bytes(&self, count: usize) {
        self.transaction_in_mempool_size_bytes.record(count);
    }

    pub(crate) fn set_transactions_in_mempool_total(&self, count: usize) {
        self.transactions_in_mempool_total.set(count);
    }

    pub(crate) fn increment_mempool_recosted(&self) {
        self.mempool_recosted.increment(1);
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

        let check_tx_removed_too_large = builder
            .new_counter_factory(
                CHECK_TX_REMOVED_TOO_LARGE,
                "The number of transactions that have been removed from the mempool due to being \
                 too large",
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

        let check_tx_duration_seconds_convert_address = builder
            .new_histogram_factory(
                CHECK_TX_DURATION_SECONDS_CONVERT_ADDRESS,
                "The amount of time taken in seconds to convert an address",
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

        let check_tx_removed_failed_stateless = builder
            .new_counter_factory(
                CHECK_TX_REMOVED_FAILED_STATELESS,
                "The number of transactions that have been removed from the mempool due to \
                 failing the stateless check",
            )?
            .register()?;

        let check_tx_removed_stale_nonce = builder
            .new_counter_factory(
                CHECK_TX_REMOVED_STALE_NONCE,
                "The number of transactions that have been removed from the mempool due to having \
                 a stale nonce",
            )?
            .register()?;

        let mut check_tx_duration_factory = builder.new_histogram_factory(
            CHECK_TX_DURATION_SECONDS,
            "The amount of time taken in seconds to successfully complete the various stages of \
             check_tx",
        )?;
        let check_tx_duration_seconds_parse_tx = check_tx_duration_factory.register_with_labels(
            &[(CHECK_TX_STAGE, "length check and parse raw tx".to_string())],
        )?;
        let check_tx_duration_seconds_check_stateless = check_tx_duration_factory
            .register_with_labels(&[(CHECK_TX_STAGE, "stateless check".to_string())])?;
        let check_tx_duration_seconds_check_nonce = check_tx_duration_factory
            .register_with_labels(&[(CHECK_TX_STAGE, "nonce check".to_string())])?;
        let check_tx_duration_seconds_check_chain_id = check_tx_duration_factory
            .register_with_labels(&[(CHECK_TX_STAGE, "chain id check".to_string())])?;
        let check_tx_duration_seconds_check_removed = check_tx_duration_factory
            .register_with_labels(&[(CHECK_TX_STAGE, "check for removal".to_string())])?;
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

        let mempool_recosted = builder
            .new_counter_factory(
                MEMPOOL_RECOSTED,
                "The number of times the mempool has been recosted",
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
            check_tx_removed_too_large,
            check_tx_removed_expired,
            check_tx_removed_failed_execution,
            check_tx_removed_failed_stateless,
            check_tx_removed_stale_nonce,
            check_tx_duration_seconds_parse_tx,
            check_tx_duration_seconds_check_stateless,
            check_tx_duration_seconds_check_nonce,
            check_tx_duration_seconds_check_chain_id,
            check_tx_duration_seconds_check_removed,
            check_tx_duration_seconds_convert_address,
            check_tx_duration_seconds_fetch_balances,
            check_tx_duration_seconds_fetch_tx_cost,
            check_tx_duration_seconds_insert_to_app_mempool,
            actions_per_transaction_in_mempool,
            transaction_in_mempool_size_bytes,
            transactions_in_mempool_total,
            mempool_recosted,
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
    CHECK_TX_REMOVED_TOO_LARGE,
    CHECK_TX_REMOVED_EXPIRED,
    CHECK_TX_REMOVED_FAILED_EXECUTION,
    CHECK_TX_REMOVED_FAILED_STATELESS,
    CHECK_TX_REMOVED_STALE_NONCE,
    CHECK_TX_REMOVED_ACCOUNT_BALANCE,
    CHECK_TX_DURATION_SECONDS,
    CHECK_TX_DURATION_SECONDS_CONVERT_ADDRESS,
    CHECK_TX_DURATION_SECONDS_FETCH_BALANCES,
    CHECK_TX_DURATION_SECONDS_FETCH_TX_COST,
    ACTIONS_PER_TRANSACTION_IN_MEMPOOL,
    TRANSACTION_IN_MEMPOOL_SIZE_BYTES,
    TRANSACTIONS_IN_MEMPOOL_TOTAL,
    MEMPOOL_RECOSTED
);

#[cfg(test)]
mod tests {
    use super::{
        ACTIONS_PER_TRANSACTION_IN_MEMPOOL,
        CHECK_TX_DURATION_SECONDS,
        CHECK_TX_REMOVED_ACCOUNT_BALANCE,
        CHECK_TX_REMOVED_EXPIRED,
        CHECK_TX_REMOVED_FAILED_EXECUTION,
        CHECK_TX_REMOVED_FAILED_STATELESS,
        CHECK_TX_REMOVED_STALE_NONCE,
        CHECK_TX_REMOVED_TOO_LARGE,
        MEMPOOL_RECOSTED,
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
        assert_const(CHECK_TX_REMOVED_TOO_LARGE, "check_tx_removed_too_large");
        assert_const(CHECK_TX_REMOVED_EXPIRED, "check_tx_removed_expired");
        assert_const(
            CHECK_TX_REMOVED_FAILED_EXECUTION,
            "check_tx_removed_failed_execution",
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
        assert_const(MEMPOOL_RECOSTED, "mempool_recosted");
    }
}
