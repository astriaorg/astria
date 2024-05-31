//! Crate-specific metrics functionality.
//!
//! Registers metrics & lists constants to be used as metric names throughout crate.

use metrics::{
    describe_counter,
    describe_histogram,
    Unit,
};

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

pub const BLOBS_PER_CELESTIA_FETCH: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_blobs_per_celestia_fetch",);

pub const CELESTIA_BLOB_FETCH_ERROR_COUNT: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_celestia_blob_fetch_error_count");

pub const DECODED_ITEMS_PER_CELESTIA_FETCH: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_decoded_items_per_celestia_fetch",
);

pub const SEQUENCER_BLOCKS_METADATA_VERIFIED_PER_CELESTIA_FETCH: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_sequencer_blocks_metadata_verified_per_celestia_fetch",
);

pub const SEQUENCER_BLOCK_INFORMATION_RECONSTRUCTED_PER_CELESTIA_FETCH: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "_sequencer_block_information_reconstructed_per_celestia_fetch",
);

pub const EXECUTED_FIRM_BLOCK_NUMBER: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_executed_firm_block_number");
pub const EXECUTED_SOFT_BLOCK_NUMBER: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_executed_soft_block_number");

pub const TRANSACTIONS_PER_EXECUTED_BLOCK: &str =
    concat!(env!("CARGO_CRATE_NAME"), "_transactions_per_executed_block",);
