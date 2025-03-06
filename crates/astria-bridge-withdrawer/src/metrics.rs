use std::time::Duration;
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
    current_nonce: Gauge,
    nonce_fetch_count: Counter,
    nonce_fetch_failure_count: Counter,
    nonce_fetch_latency: Histogram,
    sequencer_submission_failure_count: Counter,
    sequencer_submission_latency: Histogram,
    batch_settled_value: Histogram,
}

impl Metrics {
    pub(crate) fn set_current_nonce(&self, nonce: u32) {
        self.current_nonce.set(nonce);
    }

    pub(crate) fn increment_nonce_fetch_count(&self) {
        self.nonce_fetch_count.increment(1);
    }

    pub(crate) fn increment_nonce_fetch_failure_count(&self) {
        self.nonce_fetch_failure_count.increment(1);
    }

    pub(crate) fn record_nonce_fetch_latency(&self, latency: Duration) {
        self.nonce_fetch_latency.record(latency);
    }

    pub(crate) fn record_sequencer_submission_latency(&self, latency: Duration) {
        self.sequencer_submission_latency.record(latency);
    }

    pub(crate) fn increment_sequencer_submission_failure_count(&self) {
        self.sequencer_submission_failure_count.increment(1);
    }

    pub(crate) fn record_batch_settled_value(&self, value: u128) {
        self.batch_settled_value.record(value);
    }
}

impl metrics::Metrics for Metrics {
    type Config = crate::Config;

    fn register(
        builder: &mut RegisteringBuilder,
        config: &Self::Config,
    ) -> Result<Self, metrics::Error> {
        let current_nonce = builder
            .new_gauge_factory(CURRENT_NONCE, "The current nonce")?
            .register()?;

        let nonce_fetch_count = builder
            .new_counter_factory(
                NONCE_FETCH_COUNT,
                "The number of times a nonce was fetched from the sequencer",
            )?
            .register()?;

        let nonce_fetch_failure_count = builder
            .new_counter_factory(
                NONCE_FETCH_FAILURE_COUNT,
                "The number of failed attempts to fetch nonce from sequencer",
            )?
            .register()?;

        let nonce_fetch_latency = builder
            .new_histogram_factory(
                NONCE_FETCH_LATENCY,
                "The latency of fetching nonce from sequencer",
            )?
            .register()?;

        let sequencer_submission_failure_count = builder
            .new_counter_factory(
                SEQUENCER_SUBMISSION_FAILURE_COUNT,
                "The number of failed transaction submissions to the sequencer",
            )?
            .register()?;

        let sequencer_submission_latency = builder
            .new_histogram_factory(
                SEQUENCER_SUBMISSION_LATENCY,
                "The latency of submitting a transaction to the sequencer",
            )?
            .register()?;

        let denom_name = format!("{}", config.rollup_asset_denomination);
        let batch_settled_value = builder
            .new_histogram_factory(
                BATCH_SETTLED_VALUE,
                "Total value of withdrawals settled in a given sequencer block",
            )?
            .register_with_labels(&[("denom", denom_name)])?;

        Ok(Self {
            current_nonce,
            nonce_fetch_count,
            nonce_fetch_failure_count,
            nonce_fetch_latency,
            sequencer_submission_failure_count,
            sequencer_submission_latency,
            batch_settled_value,
        })
    }
}

metric_names!(const METRICS_NAMES:
    NONCE_FETCH_COUNT,
    NONCE_FETCH_FAILURE_COUNT,
    NONCE_FETCH_LATENCY,
    CURRENT_NONCE,
    SEQUENCER_SUBMISSION_FAILURE_COUNT,
    SEQUENCER_SUBMISSION_LATENCY,
    BATCH_SETTLED_VALUE,
);

#[cfg(test)]
mod tests {
    use super::{
        BATCH_SETTLED_VALUE,
        CURRENT_NONCE,
        NONCE_FETCH_COUNT,
        NONCE_FETCH_FAILURE_COUNT,
        NONCE_FETCH_LATENCY,
        SEQUENCER_SUBMISSION_FAILURE_COUNT,
        SEQUENCER_SUBMISSION_LATENCY,
    };

    #[track_caller]
    fn assert_const(actual: &'static str, suffix: &str) {
        // XXX: hard-code this so the crate name isn't accidentally changed.
        const CRATE_NAME: &str = "astria_bridge_withdrawer";
        let expected = format!("{CRATE_NAME}_{suffix}");
        assert_eq!(expected, actual);
    }

    #[test]
    fn metrics_are_as_expected() {
        assert_const(CURRENT_NONCE, "current_nonce");
        assert_const(NONCE_FETCH_COUNT, "nonce_fetch_count");
        assert_const(NONCE_FETCH_FAILURE_COUNT, "nonce_fetch_failure_count");
        assert_const(NONCE_FETCH_LATENCY, "nonce_fetch_latency");
        assert_const(
            SEQUENCER_SUBMISSION_FAILURE_COUNT,
            "sequencer_submission_failure_count",
        );
        assert_const(SEQUENCER_SUBMISSION_LATENCY, "sequencer_submission_latency");
        assert_const(BATCH_SETTLED_VALUE, "batch_settled_value");
    }
}
