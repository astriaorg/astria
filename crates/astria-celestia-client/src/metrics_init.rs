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
    gauge!(ROLLUP_BLOBS_PER_ASTRIA_BLOCK, "lib" => env!("CARGO_CRATE_NAME"));
    describe_gauge!(
        ROLLUP_BLOBS_PER_ASTRIA_BLOCK,
        Unit::Count,
        "The number of rollup blobs generated from a single astria sequencer block"
    );

    gauge!(ROLLUP_BLOBS_PER_CELESTIA_TX, "lib" => env!("CARGO_CRATE_NAME"));
    describe_gauge!(
        ROLLUP_BLOBS_PER_CELESTIA_TX,
        Unit::Count,
        "The total number of rollup blobs included in the last Celestia submission"
    );
}

pub const ROLLUP_BLOBS_PER_ASTRIA_BLOCK: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_rollups_blobs_per_astria_block");
pub const ROLLUP_BLOBS_PER_CELESTIA_TX: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_rollup_blobs_per_celestia_tx");
