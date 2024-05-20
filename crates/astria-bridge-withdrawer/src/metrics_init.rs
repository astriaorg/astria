//! Crate-specific metrics functionality.
//!
//! Registers metrics & lists constants to be used as metric names throughout crate.

use metrics::{
    describe_counter,
    describe_histogram,
    Unit,
};

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
}

pub const NONCE_FETCH_COUNT: &str = concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_count");

pub const NONCE_FETCH_FAILURE_COUNT: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_failure_count");

pub const NONCE_FETCH_LATENCY: &str = concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_latency");
