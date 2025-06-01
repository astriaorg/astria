use telemetry::{
    metric_names,
    metrics::{
        Counter,
        Histogram,
        Recorder,
        RegisteringBuilder,
    },
};

const NAMESPACE_TYPE_LABEL: &str = "namespace_type";

pub struct Metrics {
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
    pub(crate) fn record_metadata_blobs_per_celestia_fetch(&self, blob_count: usize) {
        self.metadata_blobs_per_celestia_fetch.record(blob_count);
    }

    pub(crate) fn record_rollup_data_blobs_per_celestia_fetch(&self, blob_count: usize) {
        self.rollup_data_blobs_per_celestia_fetch.record(blob_count);
    }

    pub(crate) fn increment_celestia_blob_fetch_error_count(&self) {
        self.celestia_blob_fetch_error_count.increment(1);
    }

    pub(crate) fn record_decoded_metadata_items_per_celestia_fetch(&self, item_count: usize) {
        self.decoded_metadata_items_per_celestia_fetch
            .record(item_count);
    }

    pub(crate) fn record_decoded_rollup_data_items_per_celestia_fetch(&self, item_count: usize) {
        self.decoded_rollup_data_items_per_celestia_fetch
            .record(item_count);
    }

    pub(crate) fn record_sequencer_blocks_metadata_verified_per_celestia_fetch(
        &self,
        block_count: usize,
    ) {
        self.sequencer_blocks_metadata_verified_per_celestia_fetch
            .record(block_count);
    }

    pub(crate) fn record_sequencer_block_information_reconstructed_per_celestia_fetch(
        &self,
        block_count: usize,
    ) {
        self.sequencer_block_information_reconstructed_per_celestia_fetch
            .record(block_count);
    }

    pub(crate) fn absolute_set_executed_firm_block_number(&self, block_number: u64) {
        self.executed_firm_block_number.absolute(block_number);
    }

    pub(crate) fn absolute_set_executed_soft_block_number(&self, block_number: u64) {
        self.executed_soft_block_number.absolute(block_number);
    }

    pub(crate) fn record_transactions_per_executed_block(&self, tx_count: usize) {
        self.transactions_per_executed_block.record(tx_count);
    }
}

impl telemetry::Metrics for Metrics {
    type Config = ();

    fn register<R: Recorder>(
        builder: &mut RegisteringBuilder<R>,
        _config: &Self::Config,
    ) -> Result<Self, telemetry::metrics::Error> {
        let metadata = "metadata".to_string();
        let rollup_data = "rollup_data".to_string();

        let mut factory = builder.new_histogram_factory(
            BLOBS_PER_CELESTIA_FETCH,
            "The number of Celestia blobs received per request sent",
        )?;
        let metadata_blobs_per_celestia_fetch =
            factory.register_with_labels(&[(NAMESPACE_TYPE_LABEL, metadata.clone())])?;
        let rollup_data_blobs_per_celestia_fetch =
            factory.register_with_labels(&[(NAMESPACE_TYPE_LABEL, rollup_data.clone())])?;

        let celestia_blob_fetch_error_count = builder
            .new_counter_factory(
                CELESTIA_BLOB_FETCH_ERROR_COUNT,
                "The number of calls made to fetch a blob from Celestia which have failed",
            )?
            .register()?;

        let mut factory = builder.new_histogram_factory(
            DECODED_ITEMS_PER_CELESTIA_FETCH,
            "The number of items decoded from the Celestia blobs received per request sent",
        )?;
        let decoded_metadata_items_per_celestia_fetch =
            factory.register_with_labels(&[(NAMESPACE_TYPE_LABEL, metadata)])?;
        let decoded_rollup_data_items_per_celestia_fetch =
            factory.register_with_labels(&[(NAMESPACE_TYPE_LABEL, rollup_data)])?;

        let sequencer_blocks_metadata_verified_per_celestia_fetch = builder
            .new_histogram_factory(
                SEQUENCER_BLOCKS_METADATA_VERIFIED_PER_CELESTIA_FETCH,
                "The number of sequencer blocks in a single Celestia blob fetch whose metadata \
                 was verified",
            )?
            .register()?;

        let sequencer_block_information_reconstructed_per_celestia_fetch = builder
            .new_histogram_factory(
                SEQUENCER_BLOCK_INFORMATION_RECONSTRUCTED_PER_CELESTIA_FETCH,
                "The number of sequencer blocks (or specifically, the subset pertaining to the \
                 rollup) reconstructed from a single Celestia blob fetch",
            )?
            .register()?;

        let executed_firm_block_number = builder
            .new_counter_factory(
                EXECUTED_FIRM_BLOCK_NUMBER,
                "The number/rollup height of the last executed or confirmed firm block",
            )?
            .register()?;

        let executed_soft_block_number = builder
            .new_counter_factory(
                EXECUTED_SOFT_BLOCK_NUMBER,
                "The number/rollup height of the last executed soft block",
            )?
            .register()?;

        let transactions_per_executed_block = builder
            .new_histogram_factory(
                TRANSACTIONS_PER_EXECUTED_BLOCK,
                "The number of transactions that were included in the latest block executed \
                 against the rollup",
            )?
            .register()?;

        Ok(Self {
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
        })
    }
}

metric_names!(const METRICS_NAMES:
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
    use super::{
        BLOBS_PER_CELESTIA_FETCH,
        CELESTIA_BLOB_FETCH_ERROR_COUNT,
        DECODED_ITEMS_PER_CELESTIA_FETCH,
        EXECUTED_FIRM_BLOCK_NUMBER,
        EXECUTED_SOFT_BLOCK_NUMBER,
        SEQUENCER_BLOCKS_METADATA_VERIFIED_PER_CELESTIA_FETCH,
        SEQUENCER_BLOCK_INFORMATION_RECONSTRUCTED_PER_CELESTIA_FETCH,
        TRANSACTIONS_PER_EXECUTED_BLOCK,
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
