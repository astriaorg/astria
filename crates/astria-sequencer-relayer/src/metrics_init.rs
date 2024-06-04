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

/// Registers all metrics used by this crate.
pub fn register() {
    describe_counter!(
        CELESTIA_SUBMISSION_COUNT,
        Unit::Count,
        "The number of calls made to submit to Celestia"
    );

    describe_counter!(
        CELESTIA_SUBMISSION_HEIGHT,
        Unit::Count,
        "The height of the last blob successfully submitted to Celestia"
    );

    describe_counter!(
        CELESTIA_SUBMISSION_FAILURE_COUNT,
        Unit::Count,
        "The number of calls made to submit to Celestia which have failed"
    );

    describe_counter!(
        SEQUENCER_BLOCK_FETCH_FAILURE_COUNT,
        Unit::Count,
        "The number of calls made to fetch a block from sequencer which have failed"
    );

    describe_counter!(
        SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT,
        Unit::Count,
        "The number of calls made to fetch the current height from sequencer which have failed"
    );

    describe_counter!(
        SEQUENCER_SUBMISSION_HEIGHT,
        Unit::Count,
        "The height of the highest sequencer block successfully submitted to Celestia"
    );

    describe_histogram!(
        BLOCKS_PER_CELESTIA_TX,
        Unit::Count,
        "The number of Astria blocks per Celestia submission"
    );

    describe_histogram!(
        BLOBS_PER_CELESTIA_TX,
        Unit::Count,
        "The number of blobs (Astria Sequencer blocks converted to Celestia blobs) per Celestia \
         submission"
    );

    describe_histogram!(
        BYTES_PER_CELESTIA_TX,
        Unit::Bytes,
        "The total number of payload bytes (Astria Sequencer blocks converted to Celestia blobs) \
         per Celestia submission"
    );

    describe_histogram!(
        CELESTIA_SUBMISSION_LATENCY,
        Unit::Seconds,
        "The time it takes to submit a blob to Celestia"
    );

    describe_histogram!(
        CELESTIA_PAYLOAD_CREATION_LATENCY,
        Unit::Microseconds,
        "The time it takes to create a new payload for submitting to Celestia (encoding to \
         protobuf, compression, creating blobs)"
    );

    describe_gauge!(
        COMPRESSION_RATIO_FOR_ASTRIA_BLOCK,
        Unit::Count,
        "Ratio of uncompressed:compressed data size for all `blob.data`s in an Astria block"
    );
}

// We configure buckets for manually, in order to ensure Prometheus metrics are structured as a
// Histogram, rather than as a Summary. These values are loosely based on the initial Summary
// output, and may need to be updated over time.
pub const HISTOGRAM_BUCKETS: &[f64; 5] = &[0.00001, 0.0001, 0.001, 0.01, 0.1];

metric_name!(pub const CELESTIA_SUBMISSION_HEIGHT);
metric_name!(pub const CELESTIA_SUBMISSION_COUNT);
metric_name!(pub const CELESTIA_SUBMISSION_FAILURE_COUNT);
metric_name!(pub const BLOCKS_PER_CELESTIA_TX);
metric_name!(pub const BLOBS_PER_CELESTIA_TX);
metric_name!(pub const BYTES_PER_CELESTIA_TX);
metric_name!(pub const CELESTIA_PAYLOAD_CREATION_LATENCY);
metric_name!(pub const CELESTIA_SUBMISSION_LATENCY);
metric_name!(pub const SEQUENCER_BLOCK_FETCH_FAILURE_COUNT);
metric_name!(pub const SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT);
metric_name!(pub const SEQUENCER_SUBMISSION_HEIGHT);
metric_name!(pub const COMPRESSION_RATIO_FOR_ASTRIA_BLOCK);

#[cfg(test)]
mod tests {
    use super::{
        BLOBS_PER_CELESTIA_TX,
        BLOCKS_PER_CELESTIA_TX,
        BYTES_PER_CELESTIA_TX,
        CELESTIA_PAYLOAD_CREATION_LATENCY,
        CELESTIA_SUBMISSION_COUNT,
        CELESTIA_SUBMISSION_FAILURE_COUNT,
        CELESTIA_SUBMISSION_HEIGHT,
        CELESTIA_SUBMISSION_LATENCY,
        COMPRESSION_RATIO_FOR_ASTRIA_BLOCK,
        SEQUENCER_BLOCK_FETCH_FAILURE_COUNT,
        SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT,
        SEQUENCER_SUBMISSION_HEIGHT,
    };

    #[track_caller]
    fn assert_const(actual: &'static str, suffix: &str) {
        // XXX: hard-code this so the crate name isn't accidentally changed.
        const CRATE_NAME: &str = "astria_sequencer_relayer";
        let expected = format!("{CRATE_NAME}_{suffix}");
        assert_eq!(expected, actual);
    }

    #[test]
    fn metrics_are_as_expected() {
        assert_const(CELESTIA_SUBMISSION_HEIGHT, "celestia_submission_height");
        assert_const(CELESTIA_SUBMISSION_COUNT, "celestia_submission_count");
        assert_const(
            CELESTIA_SUBMISSION_FAILURE_COUNT,
            "celestia_submission_failure_count",
        );
        assert_const(BLOCKS_PER_CELESTIA_TX, "blocks_per_celestia_tx");
        assert_const(BLOBS_PER_CELESTIA_TX, "blobs_per_celestia_tx");
        assert_const(BYTES_PER_CELESTIA_TX, "bytes_per_celestia_tx");
        assert_const(
            CELESTIA_PAYLOAD_CREATION_LATENCY,
            "celestia_payload_creation_latency",
        );
        assert_const(CELESTIA_SUBMISSION_LATENCY, "celestia_submission_latency");
        assert_const(
            SEQUENCER_BLOCK_FETCH_FAILURE_COUNT,
            "sequencer_block_fetch_failure_count",
        );
        assert_const(
            SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT,
            "sequencer_height_fetch_failure_count",
        );
        assert_const(SEQUENCER_SUBMISSION_HEIGHT, "sequencer_submission_height");
        assert_const(
            COMPRESSION_RATIO_FOR_ASTRIA_BLOCK,
            "compression_ratio_for_astria_block",
        );
    }
}
