//! Crate-specific metrics functionality.
//!
//! Registers metrics & lists constants to be used as metric names throughout crate.

use metrics::{
    describe_counter,
    describe_gauge,
    describe_histogram,
    Unit,
};

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

pub const CELESTIA_SUBMISSION_HEIGHT: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_celestia_submission_height");

pub const CELESTIA_SUBMISSION_COUNT: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_celestia_submission_count");

pub const CELESTIA_SUBMISSION_FAILURE_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_celestia_submission_failure_count"
);

pub const BLOCKS_PER_CELESTIA_TX: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_blocks_per_celestia_tx");

pub const BLOBS_PER_CELESTIA_TX: &str = concat!(env!("CARGO_CRATE_NAME"), "_blobs_per_celestia_tx");

pub const BYTES_PER_CELESTIA_TX: &str = concat!(env!("CARGO_CRATE_NAME"), "_bytes_per_celestia_tx");

pub const CELESTIA_PAYLOAD_CREATION_LATENCY: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_celestia_payload_creation_latency"
);

pub const CELESTIA_SUBMISSION_LATENCY: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_celestia_submission_latency");

pub const SEQUENCER_BLOCK_FETCH_FAILURE_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_sequencer_block_fetch_failure_count",
);

pub const SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_sequencer_height_fetch_failure_count",
);

pub const SEQUENCER_SUBMISSION_HEIGHT: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_sequencer_submission_height");

pub const COMPRESSION_RATIO_FOR_ASTRIA_BLOCK: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_compression_ratio_for_astria_block"
);
