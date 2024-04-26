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
    // collectors metrics
    describe_counter!(
        TRANSACTIONS_COLLECTED,
        Unit::Count,
        "The number of transactions received by the collectors labelled by rollup and collector \
         type"
    );
    describe_counter!(
        TRANSACTIONS_DROPPED,
        Unit::Count,
        "The number of transactions dropped by the collectors before bundling it labelled by \
         rollup and collector type"
    );
    describe_counter!(
        TRANSACTIONS_FORWARDED,
        Unit::Count,
        "The number of transactions successfully sent by the collectors to be bundled labelled by \
         rollup and collector type"
    );

    // executor metrics
    describe_counter!(
        TRANSACTIONS_RECEIVED,
        Unit::Count,
        "The number of transactions successfully received from collectors and bundled labelled by \
         rollup"
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
    describe_counter!(
        BUNDLES_SUBMISSION_SUCCESS_COUNT,
        Unit::Count,
        "The number of successful bundle submissions to the sequencer"
    );
    describe_counter!(
        BUNDLES_SUBMISSION_FAILURE_COUNT,
        Unit::Count,
        "The number of failed bundle submissions to the sequencer"
    );
    describe_histogram!(
        BUNDLES_SUBMITTED_TRANSACTIONS_COUNT,
        Unit::Count,
        "The number of outgoing transactions to the sequencer"
    );
    describe_histogram!(
        BUNDLES_SUBMITTED_BYTES,
        Unit::Bytes,
        "The size of bundles in the outgoing bundle"
    );

    // bundle factory metrics
    describe_counter!(
        BUNDLES_TOTAL_COUNT,
        Unit::Count,
        "The total number of finished bundles constructed over the composer's lifetime"
    );
    describe_histogram!(
        BUNDLES_TOTAL_BYTES,
        Unit::Bytes,
        "The distribution of sizes of finished bundles constructed over the composer's lifetime"
    );
    describe_histogram!(
        BUNDLES_TOTAL_TRANSACTIONS_COUNT,
        Unit::Count,
        "The total number of transactions in finished bundles constructed over the composer's \
         lifetime"
    );
}

pub const TRANSACTIONS_COLLECTED: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_transactions_collected");

pub const TRANSACTIONS_DROPPED: &str = concat!(env!("CARGO_CRATE_NAME"), "_transactions_dropped");

pub const TRANSACTIONS_FORWARDED: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_transactions_forwarded");

pub const TRANSACTIONS_RECEIVED: &str = concat!(env!("CARGO_CRATE_NAME"), "_transactions_received");

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

pub const BUNDLES_SUBMISSION_SUCCESS_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_bundles_submission_success_count"
);

pub const BUNDLES_SUBMISSION_FAILURE_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_bundles_submission_failure_count"
);

pub const BUNDLES_SUBMITTED_TRANSACTIONS_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_bundles_submitted_transactions_count"
);

pub const BUNDLES_SUBMITTED_BYTES: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_bundles_submitted_bytes");

pub const BUNDLES_TOTAL_COUNT: &str = concat!(env!("CARGO_CRATE_NAME"), "_bundles_total_count");

pub const BUNDLES_TOTAL_BYTES: &str = concat!(env!("CARGO_CRATE_NAME"), "_bundles_total_bytes");

pub const BUNDLES_TOTAL_TRANSACTIONS_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_bundles_total_transactions_count"
);
