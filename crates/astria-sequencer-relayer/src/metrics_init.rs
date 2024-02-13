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
    celestia_client::metrics_init::register();

    describe_counter!(
        CELESTIA_SUBMISSION_HEIGHT,
        Unit::Count,
        "The height of the last blob submitted to Celestia"
    );

    describe_counter!(
        CELESTIA_SUBMISSION_COUNT,
        Unit::Count,
        "The number of calls made to submit to celestia"
    );

    describe_counter!(
        CELESTIA_SUBMISSION_COUNT,
        Unit::Count,
        "The number of calls made to submit to celestia which have failed"
    );

    describe_gauge!(
        BLOCKS_PER_CELESTIA_TX,
        Unit::Count,
        "The number of astria blocks included in the last Celestia submission"
    );

    describe_histogram!(
        CELESTIA_SUBMISSION_LATENCY,
        Unit::Seconds,
        "The time it takes to submit a blob to Celestia"
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
pub const CELESTIA_SUBMISSION_LATENCY: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_celestia_submission_latency");
