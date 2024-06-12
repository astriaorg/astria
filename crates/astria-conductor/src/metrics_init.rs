//! Crate-specific metrics functionality.
//!
//! Registers metrics & lists constants to be used as metric names throughout crate.

use metrics::{
    describe_counter,
    describe_histogram,
    Unit,
};
use telemetry::metric_names;

pub(crate) const NAMESPACE_TYPE_LABEL: &str = "namespace_type";
pub(crate) const NAMESPACE_TYPE_METADATA: &str = "metadata";
pub(crate) const NAMESPACE_TYPE_ROLLUP_DATA: &str = "rollup_data";

pub fn register() {
    describe_histogram!(
        BLOBS_PER_CELESTIA_FETCH,
        Unit::Count,
        "The number of Celestia blobs received per request sent"
    );

    describe_counter!(
        CELESTIA_BLOB_FETCH_ERROR_COUNT,
        Unit::Count,
        "The number of calls made to fetch a blob from Celestia which have failed"
    );

    describe_histogram!(
        DECODED_ITEMS_PER_CELESTIA_FETCH,
        Unit::Count,
        "The number of items decoded from the Celestia blobs received per request sent"
    );

    describe_counter!(
        EXECUTED_FIRM_BLOCK_NUMBER,
        Unit::Count,
        "The number/rollup height of the last executed or confirmed firm block"
    );

    describe_counter!(
        EXECUTED_SOFT_BLOCK_NUMBER,
        Unit::Count,
        "The number/rollup height of the last executed soft block"
    );

    describe_histogram!(
        SEQUENCER_BLOCK_INFORMATION_RECONSTRUCTED_PER_CELESTIA_FETCH,
        Unit::Count,
        "The number of sequencer blocks (or specifically, the subset pertaining to the rollup) \
         reconstructed from a single Celestia blob fetch"
    );

    describe_histogram!(
        TRANSACTIONS_PER_EXECUTED_BLOCK,
        Unit::Count,
        "The number of transactions that were included in the latest block executed against the \
         rollup"
    );
}

metric_names!(pub const METRICS_NAMES:
    BLOBS_PER_CELESTIA_FETCH,
    CELESTIA_BLOB_FETCH_ERROR_COUNT,
    DECODED_ITEMS_PER_CELESTIA_FETCH,
    SEQUENCER_BLOCKS_METADATA_VERIFIED_PER_CELESTIA_FETCH,
    SEQUENCER_BLOCK_INFORMATION_RECONSTRUCTED_PER_CELESTIA_FETCH,

    EXECUTED_FIRM_BLOCK_NUMBER,
    EXECUTED_SOFT_BLOCK_NUMBER,
    TRANSACTIONS_PER_EXECUTED_BLOCK
);

#[cfg(test)]
mod tests {
    use super::TRANSACTIONS_PER_EXECUTED_BLOCK;
    use crate::metrics_init::{
        BLOBS_PER_CELESTIA_FETCH,
        CELESTIA_BLOB_FETCH_ERROR_COUNT,
        DECODED_ITEMS_PER_CELESTIA_FETCH,
        EXECUTED_FIRM_BLOCK_NUMBER,
        EXECUTED_SOFT_BLOCK_NUMBER,
        SEQUENCER_BLOCKS_METADATA_VERIFIED_PER_CELESTIA_FETCH,
        SEQUENCER_BLOCK_INFORMATION_RECONSTRUCTED_PER_CELESTIA_FETCH,
    };

    #[track_caller]
    fn assert_const(actual: &'static str, suffix: &str) {
        // XXX: hard-code this so the crate name isn't accidentally changed.
        const CRATE_NAME: &str = "astria_conductor";
        let expected = format!("{CRATE_NAME}_{suffix}");
        assert_eq!(expected, actual);
    }

    #[test]
    fn metrics_are_as_expected() {
        assert_const(BLOBS_PER_CELESTIA_FETCH, "blobs_per_celestia_fetch");
        assert_const(
            CELESTIA_BLOB_FETCH_ERROR_COUNT,
            "celestia_blob_fetch_error_count",
        );
        assert_const(
            DECODED_ITEMS_PER_CELESTIA_FETCH,
            "decoded_items_per_celestia_fetch",
        );

        assert_const(
            SEQUENCER_BLOCKS_METADATA_VERIFIED_PER_CELESTIA_FETCH,
            "sequencer_blocks_metadata_verified_per_celestia_fetch",
        );

        assert_const(
            SEQUENCER_BLOCK_INFORMATION_RECONSTRUCTED_PER_CELESTIA_FETCH,
            "sequencer_block_information_reconstructed_per_celestia_fetch",
        );
        assert_const(EXECUTED_FIRM_BLOCK_NUMBER, "executed_firm_block_number");
        assert_const(EXECUTED_SOFT_BLOCK_NUMBER, "executed_soft_block_number");
        assert_const(
            TRANSACTIONS_PER_EXECUTED_BLOCK,
            "transactions_per_executed_block",
        );
    }
}
