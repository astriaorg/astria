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
    nonce_fetch_count: Counter,
    nonce_fetch_failure_count: Counter,
    nonce_fetch_latency: Histogram,
    current_nonce: Gauge,
    sequencer_submission_failure_count: Counter,
    sequencer_submission_latency: Histogram,
}

impl Metrics {
    pub(crate) fn increment_nonce_fetch_count(&self) {
        self.nonce_fetch_count.increment(1);
    }

    pub(crate) fn increment_nonce_fetch_failure_count(&self) {
        self.nonce_fetch_failure_count.increment(1);
    }

    pub(crate) fn record_nonce_fetch_latency(&self, latency: Duration) {
        self.nonce_fetch_latency.record(latency);
    }

    pub(crate) fn set_current_nonce(&self, nonce: u32) {
        self.current_nonce.set(nonce);
    }

    pub(crate) fn record_sequencer_submission_latency(&self, latency: Duration) {
        self.sequencer_submission_latency.record(latency);
    }

    pub(crate) fn increment_sequencer_submission_failure_count(&self) {
        self.sequencer_submission_failure_count.increment(1);
    }
}

impl metrics::Metrics for Metrics {
    type Config = ();

    fn register(
        builder: &mut RegisteringBuilder,
        _config: &Self::Config,
    ) -> Result<Self, metrics::Error> {
        let nonce_fetch_count = builder
            .new_counter_factory(
                NONCE_FETCH_COUNT,
                "The number of times we have attempted to fetch the nonce",
            )?
            .register()?;

        let nonce_fetch_failure_count = builder
            .new_counter_factory(
                NONCE_FETCH_FAILURE_COUNT,
                "The number of times we have failed to fetch the nonce",
            )?
            .register()?;

        let nonce_fetch_latency = builder
            .new_histogram_factory(NONCE_FETCH_LATENCY, "The latency of nonce fetch")?
            .register()?;

        let current_nonce = builder
            .new_gauge_factory(CURRENT_NONCE, "The current nonce")?
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

        Ok(Self {
            nonce_fetch_count,
            nonce_fetch_failure_count,
            nonce_fetch_latency,
            current_nonce,
            sequencer_submission_failure_count,
            sequencer_submission_latency,
        })
    }
}

metric_names!(const METRICS_NAMES:
    NONCE_FETCH_COUNT,
    NONCE_FETCH_FAILURE_COUNT,
    NONCE_FETCH_LATENCY,
    CURRENT_NONCE,
    SEQUENCER_SUBMISSION_FAILURE_COUNT,
    SEQUENCER_SUBMISSION_LATENCY
);

#[cfg(test)]
mod tests {
    use super::{
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
    }
}
