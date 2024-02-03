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
pub fn register() {
    register_gauge!(ROLLUP_BLOBS_PER_ASTRIA_BLOCK, "library" => "astria_celestia_client");
    describe_gauge!(
        ROLLUP_BLOBS_PER_ASTRIA_BLOCK,
        Unit::Count,
        "The number of rollup blobs generated from a single astria sequencer block"
    );

    register_gauge!(ROLLUP_BLOBS_PER_CELESTIA_TX, "library" => "astria_celestia_client");
    describe_gauge!(
        ROLLUP_BLOBS_PER_CELESTIA_TX,
        Unit::Count,
        "The total number of rollup blobs included in the last Celestia submission"
    );
}

pub const ROLLUP_BLOBS_PER_ASTRIA_BLOCK: &str =
    "astria_celestia_client_rollups_blobs_per_astria_block";
pub const ROLLUP_BLOBS_PER_CELESTIA_TX: &str =
    "astria_celestia_client_rollup_blobs_per_celestia_tx";
