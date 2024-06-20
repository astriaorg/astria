use telemetry::{
    metric_names,
    metrics::{
        self,
        Counter,
        Gauge,
        Histogram,
        RegisteringBuilder,
    },
};

pub struct Metrics {
    prepare_proposal_excluded_transactions_decode_failure: Counter,
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
    check_tx_removed_account_balance: Counter,
}

impl Metrics {
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

    pub(crate) fn increment_check_tx_removed_account_balance(&self) {
        self.check_tx_removed_account_balance.increment(1);
    }
}

impl metrics::Metrics for Metrics {
    type Config = ();

    // allow: this is reasonable as we have a lot of metrics to register; the function is not
    // complex, just long.
    #[allow(clippy::too_many_lines)]
    fn register(
        builder: &mut RegisteringBuilder,
        _config: &Self::Config,
    ) -> Result<Self, metrics::Error> {
        let prepare_proposal_excluded_transactions_decode_failure = builder
            .new_counter_factory(
                PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_DECODE_FAILURE,
                "The number of transactions that have been excluded from blocks due to failing to \
                 decode",
            )?
            .register()?;

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

        let check_tx_removed_account_balance = builder
            .new_counter_factory(
                CHECK_TX_REMOVED_ACCOUNT_BALANCE,
                "The number of transactions that have been removed from the mempool due to having \
                 not enough account balance",
            )?
            .register()?;

        Ok(Self {
            prepare_proposal_excluded_transactions_decode_failure,
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
            check_tx_removed_account_balance,
        })
    }
}

metric_names!(const METRICS_NAMES:
    PREPARE_PROPOSAL_EXCLUDED_TRANSACTIONS_DECODE_FAILURE,
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
);

#[cfg(test)]
mod tests {
    use super::{
        CHECK_TX_REMOVED_ACCOUNT_BALANCE,
        CHECK_TX_REMOVED_EXPIRED,
        CHECK_TX_REMOVED_FAILED_EXECUTION,
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
    }
}
