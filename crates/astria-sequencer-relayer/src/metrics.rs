//! Crate-specific metrics functionality.
//!
//! This module re-exports the contents of the `metrics` crate.  This is
//! effectively a way to monkey-patch the functions in this module into the
//! `metrics` crate, at least from the point of view of the other code in this
//! crate.
//!
//! Code in this crate that wants to use metrics should `use crate::metrics;`,
//! so that this module shadows the `metrics` crate.
//!
//! This trick is probably good to avoid in general, because it could be
//! confusing, but in this limited case, it seems like a clean option.

pub use metrics::*;

/// Registers all metrics used by this crate.
pub fn register_metrics() {
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
