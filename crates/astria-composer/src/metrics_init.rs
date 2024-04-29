//! Crate-specific metrics functionality.
//!
//! Registers metrics & lists constants to be used as metric names throughout crate.

use metrics::{
    describe_counter,
    describe_gauge,
    describe_histogram,
    Unit,
};

/// Labels
pub(crate) const ROLLUP_ID_LABEL: &str = "rollup_id";
pub(crate) const COLLECTOR_TYPE_LABEL: &str = "collector_type";

/// Registers all metrics used by this crate.
#[allow(clippy::too_many_lines)]
pub fn register() {
    describe_counter!(
        TRANSACTIONS_RECEIVED,
        Unit::Count,
        "The number of transactions successfully received from collectors and bundled labelled by \
         rollup"
    );
    describe_counter!(
        TRANSACTIONS_DROPPED,
        Unit::Count,
        "The number of transactions dropped by the collectors before bundling it labelled by \
         rollup and collector type"
    );
    describe_counter!(
        TRANSACTIONS_DROPPED_TOO_LARGE,
        Unit::Count,
        "The number of transactions dropped because they were too large"
    );
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
    describe_histogram!(
        TRANSACTIONS_PER_SUBMISSION,
        Unit::Count,
        "The number of rollup transactions successfully sent to the sequencer in a single \
         submission"
    );
    describe_histogram!(
        BYTES_PER_SUBMISSION,
        Unit::Bytes,
        "The total bytes successfully sent to the sequencer in a single submission"
    );
}

pub const TRANSACTIONS_RECEIVED: &str = concat!(env!("CARGO_CRATE_NAME"), "_transactions_received");

pub const TRANSACTIONS_DROPPED: &str = concat!(env!("CARGO_CRATE_NAME"), "_transactions_dropped");

pub const TRANSACTIONS_DROPPED_TOO_LARGE: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_transactions_dropped_too_large");

pub const NONCE_FETCH_COUNT: &str = concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_count");

pub const NONCE_FETCH_FAILURE_COUNT: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_failure_count");

pub const NONCE_FETCH_LATENCY: &str = concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_latency");

pub const CURRENT_NONCE: &str = concat!(env!("CARGO_CRATE_NAME"), "_current_nonce");

pub const SEQUENCER_SUBMISSION_LATENCY: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_sequencer_submission_latency");

pub const SEQUENCER_SUBMISSION_FAILURE_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_sequencer_submission_failure_count"
);

pub const TRANSACTIONS_PER_SUBMISSION: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_transaction_per_submission");

pub const BYTES_PER_SUBMISSION: &str = concat!(env!("CARGO_CRATE_NAME"), "_bytes_per_submission");
