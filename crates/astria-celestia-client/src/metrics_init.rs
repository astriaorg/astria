//! Crate-specific metrics functionality.
//!
//! Registers metrics & lists constants to be used as metric names throughout crate.

use metrics::{
    describe_gauge,
    gauge,
    Unit,
};

/// Registers all metrics used by this crate.
pub fn register() {
    gauge!(ROLLUP_BLOBS_PER_ASTRIA_BLOCK, "lib" => "astria_celestia_client");
    describe_gauge!(
        ROLLUP_BLOBS_PER_ASTRIA_BLOCK,
        Unit::Count,
        "The number of rollup blobs generated from a single astria sequencer block"
    );

    gauge!(ROLLUP_BLOBS_PER_CELESTIA_TX, "lib" => "astria_celestia_client");
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
