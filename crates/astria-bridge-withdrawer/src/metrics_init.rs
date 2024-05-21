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
}

pub const NONCE_FETCH_COUNT: &str = concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_count");

pub const NONCE_FETCH_FAILURE_COUNT: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_failure_count");

pub const NONCE_FETCH_LATENCY: &str = concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_latency");

pub const CURRENT_NONCE: &str = concat!(env!("CARGO_CRATE_NAME"), "_current_nonce");

pub const SEQUENCER_SUBMISSION_FAILURE_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_sequencer_submission_failure_count"
);

pub const SEQUENCER_SUBMISSION_LATENCY: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_sequencer_submission_latency");
