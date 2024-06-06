//! Crate-specific metrics functionality.
//!
//! Registers metrics & lists constants to be used as metric names throughout crate.

use metrics::{
    describe_counter,
    describe_gauge,
    describe_histogram,
    Unit,
};
use telemetry::metric_name;

/// Labels
pub(crate) const ROLLUP_ID_LABEL: &str = "rollup_id";
pub(crate) const COLLECTOR_TYPE_LABEL: &str = "collector_type";

/// Registers all metrics used by this crate.
// allow: refactor this. being tracked in https://github.com/astriaorg/astria/issues/1027
#[allow(clippy::too_many_lines)]
pub fn register() {
    describe_counter!(
        TRANSACTIONS_RECEIVED,
        Unit::Count,
        "The number of transactions successfully received from collectors and bundled labelled by \
         rollup"
    );
    describe_counter!(
        TRANSACTIONS_DROPPED,
        Unit::Count,
        "The number of transactions dropped by the collectors before bundling it labelled by \
         rollup and collector type"
    );
    describe_counter!(
        TRANSACTIONS_DROPPED_TOO_LARGE,
        Unit::Count,
        "The number of transactions dropped because they were too large"
    );
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
    describe_histogram!(
        TRANSACTIONS_PER_SUBMISSION,
        Unit::Count,
        "The number of rollup transactions successfully sent to the sequencer in a single \
         submission"
    );
    describe_histogram!(
        BYTES_PER_SUBMISSION,
        Unit::Bytes,
        "The total bytes successfully sent to the sequencer in a single submission"
    );
}

metric_name!(pub const TRANSACTIONS_RECEIVED);
metric_name!(pub const TRANSACTIONS_DROPPED);
metric_name!(pub const TRANSACTIONS_DROPPED_TOO_LARGE);
metric_name!(pub const NONCE_FETCH_COUNT);
metric_name!(pub const NONCE_FETCH_FAILURE_COUNT);
metric_name!(pub const NONCE_FETCH_LATENCY);
metric_name!(pub const CURRENT_NONCE);
metric_name!(pub const SEQUENCER_SUBMISSION_LATENCY);
metric_name!(pub const SEQUENCER_SUBMISSION_FAILURE_COUNT);
metric_name!(pub const TRANSACTIONS_PER_SUBMISSION);
metric_name!(pub const BYTES_PER_SUBMISSION);

#[cfg(test)]
mod tests {
    use super::{
        BYTES_PER_SUBMISSION,
        CURRENT_NONCE,
        NONCE_FETCH_COUNT,
        NONCE_FETCH_FAILURE_COUNT,
        NONCE_FETCH_LATENCY,
        SEQUENCER_SUBMISSION_FAILURE_COUNT,
        SEQUENCER_SUBMISSION_LATENCY,
        TRANSACTIONS_DROPPED,
        TRANSACTIONS_DROPPED_TOO_LARGE,
        TRANSACTIONS_PER_SUBMISSION,
        TRANSACTIONS_RECEIVED,
    };

    #[track_caller]
    fn assert_const(actual: &'static str, suffix: &str) {
        // XXX: hard-code this so the crate name isn't accidentally changed.
        const CRATE_NAME: &str = "astria_composer";
        let expected = format!("{CRATE_NAME}_{suffix}");
        assert_eq!(expected, actual);
    }

    #[test]
    fn metrics_are_as_expected() {
        assert_const(TRANSACTIONS_RECEIVED, "transactions_received");
        assert_const(TRANSACTIONS_DROPPED, "transactions_dropped");
        assert_const(
            TRANSACTIONS_DROPPED_TOO_LARGE,
            "transactions_dropped_too_large",
        );
        assert_const(NONCE_FETCH_COUNT, "nonce_fetch_count");
        assert_const(NONCE_FETCH_FAILURE_COUNT, "nonce_fetch_failure_count");
        assert_const(NONCE_FETCH_LATENCY, "nonce_fetch_latency");
        assert_const(CURRENT_NONCE, "current_nonce");
        assert_const(SEQUENCER_SUBMISSION_LATENCY, "sequencer_submission_latency");
        assert_const(
            SEQUENCER_SUBMISSION_FAILURE_COUNT,
            "sequencer_submission_failure_count",
        );
        assert_const(TRANSACTIONS_PER_SUBMISSION, "transactions_per_submission");
        assert_const(BYTES_PER_SUBMISSION, "bytes_per_submission");
    }
}
