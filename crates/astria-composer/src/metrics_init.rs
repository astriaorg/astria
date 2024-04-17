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

/// Registers all metrics used by this crate.
#[allow(clippy::too_many_lines)]
pub fn register() {
    // geth collectors metrics
    describe_counter!(
        GETH_COLLECTOR_TRANSACTIONS_COLLECTED,
        Unit::Count,
        "The number of transactions received by the geth collector labelled by rollup"
    );
    describe_counter!(
        GETH_COLLECTOR_TRANSACTIONS_DROPPED,
        Unit::Count,
        "The number of transactions dropped by the geth collector before bundling it labelled by \
         rollup"
    );
    describe_counter!(
        GETH_COLLECTOR_TRANSACTIONS_FORWARDED,
        Unit::Count,
        "The number of transactions successfully sent by the geth collector to be bundled \
         labelled by rollup"
    );
    describe_histogram!(
        GETH_COLLECTOR_CONNECTION_LATENCY,
        Unit::Milliseconds,
        "The time taken to connect to geth"
    );

    // grpc collector metrics
    describe_counter!(
        GRPC_COLLECTOR_TRANSACTIONS_COLLECTED,
        Unit::Count,
        "The number of transactions received by the grpc collector"
    );
    describe_counter!(
        GRPC_COLLECTOR_TRANSACTIONS_DROPPED,
        Unit::Count,
        "The number of transactions dropped by the grpc collector before sending it to be bundled"
    );
    describe_counter!(
        GRPC_COLLECTOR_TRANSACTIONS_FORWARDED,
        Unit::Count,
        "The number of transactions successfully sent by the grpc collector before sending it to \
         be bundled"
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
        TRANSACTION_SUBMISSION_LATENCY,
        Unit::Milliseconds,
        "The latency of submitting a transaction to the sequencer"
    );
    describe_counter!(
        TRANSACTION_SUBMISSION_FAILURE_COUNT,
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
        BUNDLES_OUTGOING_TRANSACTIONS_COUNT,
        Unit::Count,
        "The number of outgoing transactions to the sequencer"
    );
    describe_histogram!(
        BUNDLES_OUTGOING_BYTES,
        Unit::Bytes,
        "The size of bundles in the outgoing bundle"
    );
    describe_counter!(
        BUNDLES_NOT_DRAINED,
        Unit::Count,
        "The number of bundles not drained during shutdown"
    );
    describe_counter!(
        BUNDLES_DRAINED,
        Unit::Count,
        "The number of bundles drained successfully during shutdown"
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

// We configure buckets for manually, in order to ensure Prometheus metrics are structured as a
// Histogram, rather than as a Summary. These values are loosely based on the initial Summary
// output, and may need to be updated over time.
pub const HISTOGRAM_BUCKETS: &[f64; 5] = &[0.00001, 0.0001, 0.001, 0.01, 0.1];

pub const GETH_COLLECTOR_TRANSACTIONS_COLLECTED: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_geth_collector_transactions_collected"
);

pub const GETH_COLLECTOR_TRANSACTIONS_DROPPED: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_geth_collector_transactions_dropped"
);

pub const GETH_COLLECTOR_TRANSACTIONS_FORWARDED: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_geth_collector_transactions_forwarded"
);

pub const GETH_COLLECTOR_CONNECTION_LATENCY: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_geth_collector_connection_latency"
);

pub const GRPC_COLLECTOR_TRANSACTIONS_COLLECTED: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_grpc_collector_transactions_collected"
);

pub const GRPC_COLLECTOR_TRANSACTIONS_DROPPED: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_grpc_collector_transactions_dropped"
);

pub const GRPC_COLLECTOR_TRANSACTIONS_FORWARDED: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_grpc_collector_transactions_forwarded"
);

pub const TRANSACTIONS_RECEIVED: &str = concat!(env!("CARGO_CRATE_NAME"), "_transactions_received");

pub const TRANSACTIONS_DROPPED_TOO_LARGE: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_transactions_dropped_too_large");

pub const NONCE_FETCH_COUNT: &str = concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_count");

pub const NONCE_FETCH_FAILURE_COUNT: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_failure_count");

pub const NONCE_FETCH_LATENCY: &str = concat!(env!("CARGO_CRATE_NAME"), "_nonce_fetch_latency");

pub const CURRENT_NONCE: &str = concat!(env!("CARGO_CRATE_NAME"), "_current_nonce");

pub const TRANSACTION_SUBMISSION_LATENCY: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_transaction_submission_latency");

pub const TRANSACTION_SUBMISSION_FAILURE_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_transaction_submission_failure_count"
);

pub const BUNDLES_SUBMISSION_SUCCESS_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_bundles_submission_success_count"
);

pub const BUNDLES_SUBMISSION_FAILURE_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_bundles_submission_failure_count"
);

pub const BUNDLES_OUTGOING_TRANSACTIONS_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_bundles_outgoing_transactions_count"
);

pub const BUNDLES_OUTGOING_BYTES: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_bundles_outgoing_bytes");

pub const BUNDLES_NOT_DRAINED: &str = concat!(env!("CARGO_CRATE_NAME"), "_bundles_not_drained");

pub const BUNDLES_DRAINED: &str = concat!(env!("CARGO_CRATE_NAME"), "_bundles_drained");

pub const BUNDLES_TOTAL_COUNT: &str = concat!(env!("CARGO_CRATE_NAME"), "_bundles_total_count");

pub const BUNDLES_TOTAL_BYTES: &str = concat!(env!("CARGO_CRATE_NAME"), "_bundles_total_bytes");

pub const BUNDLES_TOTAL_TRANSACTIONS_COUNT: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_bundles_total_transactions_count"
);
