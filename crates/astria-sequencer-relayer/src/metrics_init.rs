//! Crate-specific metrics functionality.
//!
//! Registers metrics & lists constants to be used as metric names throughout crate.

use metrics::{
    describe_counter,
    describe_gauge,
    describe_histogram,
    register_counter,
    register_gauge,
    register_histogram,
    Unit,
};

/// Registers all metrics used by this crate.
pub fn register() {
    celestia_client::metrics_init::register();

    register_counter!(CELESTIA_SUBMISSION_HEIGHT);
    describe_counter!(
        CELESTIA_SUBMISSION_HEIGHT,
        Unit::Count,
        "The height of the last blob submitted to Celestia"
    );

    register_gauge!(BLOCKS_PER_CELESTIA_TX);
    describe_gauge!(
        BLOCKS_PER_CELESTIA_TX,
        Unit::Count,
        "The number of astria blocks included in the last Celestia submission"
    );

    register_histogram!(CELESTIA_SUBMISSION_LATENCY);
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

pub const CELESTIA_SUBMISSION_HEIGHT: &str = "astria_relayer_celestia_submission_height";
pub const BLOCKS_PER_CELESTIA_TX: &str = "astria_relayer_blocks_per_celestia_tx";
pub const CELESTIA_SUBMISSION_LATENCY: &str = "astria_relayer_celestia_submission_latency";
