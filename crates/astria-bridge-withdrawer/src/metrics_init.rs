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
        NONCE_FETCH_COUNT,
        Unit::Count,
        "The number of times we have attempted to fetch the nonce"
    );
    describe_counter!(
        NONCE_FETCH_FAILURE_COUNT,
        Unit::Count,
        "The number of times we have failed to fetch the nonce"
    );
    describe_histogram!(
        NONCE_FETCH_LATENCY,
        Unit::Milliseconds,
        "The latency of nonce fetch"
    );
    describe_gauge!(CURRENT_NONCE, Unit::Count, "The current nonce");
    describe_histogram!(
        SEQUENCER_SUBMISSION_LATENCY,
        Unit::Milliseconds,
        "The latency of submitting a transaction to the sequencer"
    );
    describe_counter!(
        SEQUENCER_SUBMISSION_FAILURE_COUNT,
        Unit::Count,
        "The number of failed transaction submissions to the sequencer"
    );
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
