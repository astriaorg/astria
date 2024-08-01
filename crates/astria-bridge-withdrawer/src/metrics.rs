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

pub(crate) struct Metrics {
    current_nonce: Gauge,
    nonce_fetch_count: Counter,
    nonce_fetch_failure_count: Counter,
    nonce_fetch_latency: Histogram,
    sequencer_submission_failure_count: Counter,
    sequencer_submission_latency: Histogram,
}

impl Metrics {
    #[must_use]
    pub(crate) fn new() -> Self {
        describe_gauge!(CURRENT_NONCE, Unit::Count, "The current nonce");
        let current_nonce = gauge!(CURRENT_NONCE);

        describe_counter!(
            NONCE_FETCH_COUNT,
            Unit::Count,
            "The number of times a nonce was fetched from the sequencer"
        );
        let nonce_fetch_count = counter!(NONCE_FETCH_COUNT);

        describe_counter!(
            NONCE_FETCH_FAILURE_COUNT,
            Unit::Count,
            "The number of failed attempts to fetch nonce from sequencer"
        );
        let nonce_fetch_failure_count = counter!(NONCE_FETCH_FAILURE_COUNT);

        describe_histogram!(
            NONCE_FETCH_LATENCY,
            Unit::Seconds,
            "The latency of fetching nonce from sequencer"
        );
        let nonce_fetch_latency = histogram!(NONCE_FETCH_LATENCY);

        describe_counter!(
            SEQUENCER_SUBMISSION_FAILURE_COUNT,
            Unit::Count,
            "The number of failed transaction submissions to the sequencer"
        );
        let sequencer_submission_failure_count = counter!(SEQUENCER_SUBMISSION_FAILURE_COUNT);

        describe_histogram!(
            SEQUENCER_SUBMISSION_LATENCY,
            Unit::Seconds,
            "The latency of submitting a transaction to the sequencer"
        );
        let sequencer_submission_latency = histogram!(SEQUENCER_SUBMISSION_LATENCY);

        Self {
            current_nonce,
            nonce_fetch_count,
            nonce_fetch_failure_count,
            nonce_fetch_latency,
            sequencer_submission_failure_count,
            sequencer_submission_latency,
        }
    }

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
}

metric_names!(pub const METRICS_NAMES:
    CURRENT_NONCE,
    NONCE_FETCH_COUNT,
    NONCE_FETCH_FAILURE_COUNT,
    NONCE_FETCH_LATENCY,
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
