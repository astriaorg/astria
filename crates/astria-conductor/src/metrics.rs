use metrics::{
    counter,
    describe_counter,
    describe_histogram,
    histogram,
    Counter,
    Histogram,
    Unit,
};
use telemetry::metric_names;

const NAMESPACE_TYPE_LABEL: &str = "namespace_type";
const NAMESPACE_TYPE_METADATA: &str = "metadata";
const NAMESPACE_TYPE_ROLLUP_DATA: &str = "rollup_data";

pub(crate) struct Metrics {
    metadata_blobs_per_celestia_fetch: Histogram,
    rollup_data_blobs_per_celestia_fetch: Histogram,
    celestia_blob_fetch_error_count: Counter,
    decoded_metadata_items_per_celestia_fetch: Histogram,
    decoded_rollup_data_items_per_celestia_fetch: Histogram,
    sequencer_blocks_metadata_verified_per_celestia_fetch: Histogram,
    sequencer_block_information_reconstructed_per_celestia_fetch: Histogram,
    executed_firm_block_number: Counter,
    executed_soft_block_number: Counter,
    transactions_per_executed_block: Histogram,
}

impl Metrics {
    #[must_use]
    pub(crate) fn new() -> Self {
        describe_histogram!(
            BLOBS_PER_CELESTIA_FETCH,
            Unit::Count,
            "The number of Celestia blobs received per request sent"
        );
        let metadata_blobs_per_celestia_fetch = histogram!(
            BLOBS_PER_CELESTIA_FETCH,
            NAMESPACE_TYPE_LABEL => NAMESPACE_TYPE_METADATA,
        );
        let rollup_data_blobs_per_celestia_fetch = histogram!(
            BLOBS_PER_CELESTIA_FETCH,
            NAMESPACE_TYPE_LABEL => NAMESPACE_TYPE_ROLLUP_DATA,
        );

        describe_counter!(
            CELESTIA_BLOB_FETCH_ERROR_COUNT,
            Unit::Count,
            "The number of calls made to fetch a blob from Celestia which have failed"
        );
        let celestia_blob_fetch_error_count = counter!(CELESTIA_BLOB_FETCH_ERROR_COUNT);

        describe_histogram!(
            DECODED_ITEMS_PER_CELESTIA_FETCH,
            Unit::Count,
            "The number of items decoded from the Celestia blobs received per request sent"
        );
        let decoded_metadata_items_per_celestia_fetch = histogram!(
            DECODED_ITEMS_PER_CELESTIA_FETCH,
            NAMESPACE_TYPE_LABEL => NAMESPACE_TYPE_METADATA,
        );
        let decoded_rollup_data_items_per_celestia_fetch = histogram!(
            DECODED_ITEMS_PER_CELESTIA_FETCH,
            NAMESPACE_TYPE_LABEL => NAMESPACE_TYPE_ROLLUP_DATA,
        );

        describe_histogram!(
            SEQUENCER_BLOCKS_METADATA_VERIFIED_PER_CELESTIA_FETCH,
            Unit::Count,
            "The number of sequencer blocks in a single Celestia blob fetch whose metadata was \
             verified"
        );
        let sequencer_blocks_metadata_verified_per_celestia_fetch =
            histogram!(SEQUENCER_BLOCKS_METADATA_VERIFIED_PER_CELESTIA_FETCH);

        describe_histogram!(
            SEQUENCER_BLOCK_INFORMATION_RECONSTRUCTED_PER_CELESTIA_FETCH,
            Unit::Count,
            "The number of sequencer blocks (or specifically, the subset pertaining to the \
             rollup) reconstructed from a single Celestia blob fetch"
        );
        let sequencer_block_information_reconstructed_per_celestia_fetch =
            histogram!(SEQUENCER_BLOCK_INFORMATION_RECONSTRUCTED_PER_CELESTIA_FETCH);

        describe_counter!(
            EXECUTED_FIRM_BLOCK_NUMBER,
            Unit::Count,
            "The number/rollup height of the last executed or confirmed firm block"
        );
        let executed_firm_block_number = counter!(EXECUTED_FIRM_BLOCK_NUMBER);

        describe_counter!(
            EXECUTED_SOFT_BLOCK_NUMBER,
            Unit::Count,
            "The number/rollup height of the last executed soft block"
        );
        let executed_soft_block_number = counter!(EXECUTED_SOFT_BLOCK_NUMBER);

        describe_histogram!(
            TRANSACTIONS_PER_EXECUTED_BLOCK,
            Unit::Count,
            "The number of transactions that were included in the latest block executed against \
             the rollup"
        );
        let transactions_per_executed_block = histogram!(TRANSACTIONS_PER_EXECUTED_BLOCK);

        Self {
            metadata_blobs_per_celestia_fetch,
            rollup_data_blobs_per_celestia_fetch,
            celestia_blob_fetch_error_count,
            decoded_metadata_items_per_celestia_fetch,
            decoded_rollup_data_items_per_celestia_fetch,
            sequencer_blocks_metadata_verified_per_celestia_fetch,
            sequencer_block_information_reconstructed_per_celestia_fetch,
            executed_firm_block_number,
            executed_soft_block_number,
            transactions_per_executed_block,
        }
    }

    pub(crate) fn record_metadata_blobs_per_celestia_fetch(&self, blob_count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.metadata_blobs_per_celestia_fetch
            .record(blob_count as f64);
    }

    pub(crate) fn record_rollup_data_blobs_per_celestia_fetch(&self, blob_count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.rollup_data_blobs_per_celestia_fetch
            .record(blob_count as f64);
    }

    pub(crate) fn increment_celestia_blob_fetch_error_count(&self) {
        self.celestia_blob_fetch_error_count.increment(1);
    }

    pub(crate) fn record_decoded_metadata_items_per_celestia_fetch(&self, item_count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.decoded_metadata_items_per_celestia_fetch
            .record(item_count as f64);
    }

    pub(crate) fn record_decoded_rollup_data_items_per_celestia_fetch(&self, item_count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.decoded_rollup_data_items_per_celestia_fetch
            .record(item_count as f64);
    }

    pub(crate) fn record_sequencer_blocks_metadata_verified_per_celestia_fetch(
        &self,
        block_count: usize,
    ) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.sequencer_blocks_metadata_verified_per_celestia_fetch
            .record(block_count as f64);
    }

    pub(crate) fn record_sequencer_block_information_reconstructed_per_celestia_fetch(
        &self,
        block_count: usize,
    ) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.sequencer_block_information_reconstructed_per_celestia_fetch
            .record(block_count as f64);
    }

    pub(crate) fn absolute_set_executed_firm_block_number(&self, block_number: u32) {
        self.executed_firm_block_number
            .absolute(u64::from(block_number));
    }

    pub(crate) fn absolute_set_executed_soft_block_number(&self, block_number: u32) {
        self.executed_soft_block_number
            .absolute(u64::from(block_number));
    }

    pub(crate) fn record_transactions_per_executed_block(&self, tx_count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.transactions_per_executed_block.record(tx_count as f64);
    }
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
    use crate::metrics::{
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
