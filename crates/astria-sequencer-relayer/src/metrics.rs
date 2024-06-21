use std::time::Duration;

use metrics::{
    counter,
    describe_counter,
    describe_gauge,
    describe_histogram,
    gauge,
    histogram,
    Counter,
    Gauge,
    Histogram,
    Unit,
};
use telemetry::metric_names;

pub(crate) struct Metrics {
    celestia_submission_height: Counter,
    celestia_submission_count: Counter,
    celestia_submission_failure_count: Counter,
    blocks_per_celestia_tx: Histogram,
    blobs_per_celestia_tx: Histogram,
    bytes_per_celestia_tx: Histogram,
    celestia_payload_creation_latency: Histogram,
    celestia_submission_latency: Histogram,
    sequencer_block_fetch_failure_count: Counter,
    sequencer_height_fetch_failure_count: Counter,
    sequencer_submission_height: Counter,
    compression_ratio_for_astria_block: Gauge,
}

impl Metrics {
    #[must_use]
    pub(crate) fn new() -> Self {
        describe_counter!(
            CELESTIA_SUBMISSION_HEIGHT,
            Unit::Count,
            "The height of the last blob successfully submitted to Celestia"
        );
        let celestia_submission_height = counter!(CELESTIA_SUBMISSION_HEIGHT);

        describe_counter!(
            CELESTIA_SUBMISSION_COUNT,
            Unit::Count,
            "The number of calls made to submit to Celestia"
        );
        let celestia_submission_count = counter!(CELESTIA_SUBMISSION_COUNT);

        describe_counter!(
            CELESTIA_SUBMISSION_FAILURE_COUNT,
            Unit::Count,
            "The number of calls made to submit to Celestia which have failed"
        );
        let celestia_submission_failure_count = counter!(CELESTIA_SUBMISSION_FAILURE_COUNT);

        describe_histogram!(
            BLOCKS_PER_CELESTIA_TX,
            Unit::Count,
            "The number of Astria blocks per Celestia submission"
        );
        let blocks_per_celestia_tx = histogram!(BLOCKS_PER_CELESTIA_TX);

        describe_histogram!(
            BLOBS_PER_CELESTIA_TX,
            Unit::Count,
            "The number of blobs (Astria Sequencer blocks converted to Celestia blobs) per \
             Celestia submission"
        );
        let blobs_per_celestia_tx = histogram!(BLOBS_PER_CELESTIA_TX);

        describe_histogram!(
            BYTES_PER_CELESTIA_TX,
            Unit::Bytes,
            "The total number of payload bytes (Astria Sequencer blocks converted to Celestia \
             blobs) per Celestia submission"
        );
        let bytes_per_celestia_tx = histogram!(BYTES_PER_CELESTIA_TX);

        describe_histogram!(
            CELESTIA_PAYLOAD_CREATION_LATENCY,
            Unit::Seconds,
            "The time it takes to create a new payload for submitting to Celestia (encoding to \
             protobuf, compression, creating blobs)"
        );
        let celestia_payload_creation_latency = histogram!(CELESTIA_PAYLOAD_CREATION_LATENCY);

        describe_histogram!(
            CELESTIA_SUBMISSION_LATENCY,
            Unit::Seconds,
            "The time it takes to submit a blob to Celestia"
        );
        let celestia_submission_latency = histogram!(CELESTIA_SUBMISSION_LATENCY);

        describe_counter!(
            SEQUENCER_BLOCK_FETCH_FAILURE_COUNT,
            Unit::Count,
            "The number of calls made to fetch a block from sequencer which have failed"
        );
        let sequencer_block_fetch_failure_count = counter!(SEQUENCER_BLOCK_FETCH_FAILURE_COUNT);

        describe_counter!(
            SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT,
            Unit::Count,
            "The number of calls made to fetch the current height from sequencer which have failed"
        );
        let sequencer_height_fetch_failure_count = counter!(SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT);

        describe_counter!(
            SEQUENCER_SUBMISSION_HEIGHT,
            Unit::Count,
            "The height of the highest sequencer block successfully submitted to Celestia"
        );
        let sequencer_submission_height = counter!(SEQUENCER_SUBMISSION_HEIGHT);

        describe_gauge!(
            COMPRESSION_RATIO_FOR_ASTRIA_BLOCK,
            Unit::Count,
            "Ratio of uncompressed:compressed data size for all `blob.data`s in an Astria block"
        );
        let compression_ratio_for_astria_block = gauge!(COMPRESSION_RATIO_FOR_ASTRIA_BLOCK);

        Self {
            celestia_submission_height,
            celestia_submission_count,
            celestia_submission_failure_count,
            blocks_per_celestia_tx,
            blobs_per_celestia_tx,
            bytes_per_celestia_tx,
            celestia_payload_creation_latency,
            celestia_submission_latency,
            sequencer_block_fetch_failure_count,
            sequencer_height_fetch_failure_count,
            sequencer_submission_height,
            compression_ratio_for_astria_block,
        }
    }

    pub(crate) fn absolute_set_celestia_submission_height(&self, height: u64) {
        self.celestia_submission_height.absolute(height);
    }

    pub(crate) fn increment_celestia_submission_count(&self) {
        self.celestia_submission_count.increment(1);
    }

    pub(crate) fn increment_celestia_submission_failure_count(&self) {
        self.celestia_submission_failure_count.increment(1);
    }

    pub(crate) fn record_blocks_per_celestia_tx(&self, block_count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.blocks_per_celestia_tx.record(block_count as f64);
    }

    pub(crate) fn record_blobs_per_celestia_tx(&self, blob_count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.blobs_per_celestia_tx.record(blob_count as f64);
    }

    pub(crate) fn record_bytes_per_celestia_tx(&self, byte_count: usize) {
        // allow: precision loss is unlikely (values too small) but also unimportant in histograms.
        #[allow(clippy::cast_precision_loss)]
        self.bytes_per_celestia_tx.record(byte_count as f64);
    }

    pub(crate) fn record_celestia_payload_creation_latency(&self, latency: Duration) {
        self.celestia_payload_creation_latency.record(latency);
    }

    pub(crate) fn record_celestia_submission_latency(&self, latency: Duration) {
        self.celestia_submission_latency.record(latency);
    }

    pub(crate) fn increment_sequencer_block_fetch_failure_count(&self) {
        self.sequencer_block_fetch_failure_count.increment(1);
    }

    pub(crate) fn increment_sequencer_height_fetch_failure_count(&self) {
        self.sequencer_height_fetch_failure_count.increment(1);
    }

    pub(crate) fn absolute_set_sequencer_submission_height(&self, height: u64) {
        self.sequencer_submission_height.absolute(height);
    }

    pub(crate) fn set_compression_ratio_for_astria_block(&self, ratio: f64) {
        self.compression_ratio_for_astria_block.set(ratio);
    }
}

metric_names!(pub const METRICS_NAMES:
    CELESTIA_SUBMISSION_HEIGHT,
    CELESTIA_SUBMISSION_COUNT,
    CELESTIA_SUBMISSION_FAILURE_COUNT,
    BLOCKS_PER_CELESTIA_TX,
    BLOBS_PER_CELESTIA_TX,
    BYTES_PER_CELESTIA_TX,
    CELESTIA_PAYLOAD_CREATION_LATENCY,
    CELESTIA_SUBMISSION_LATENCY,
    SEQUENCER_BLOCK_FETCH_FAILURE_COUNT,
    SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT,
    SEQUENCER_SUBMISSION_HEIGHT,
    COMPRESSION_RATIO_FOR_ASTRIA_BLOCK
);

#[cfg(test)]
mod tests {
    use super::{
        BLOBS_PER_CELESTIA_TX,
        BLOCKS_PER_CELESTIA_TX,
        BYTES_PER_CELESTIA_TX,
        CELESTIA_PAYLOAD_CREATION_LATENCY,
        CELESTIA_SUBMISSION_COUNT,
        CELESTIA_SUBMISSION_FAILURE_COUNT,
        CELESTIA_SUBMISSION_HEIGHT,
        CELESTIA_SUBMISSION_LATENCY,
        COMPRESSION_RATIO_FOR_ASTRIA_BLOCK,
        SEQUENCER_BLOCK_FETCH_FAILURE_COUNT,
        SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT,
        SEQUENCER_SUBMISSION_HEIGHT,
    };

    #[track_caller]
    fn assert_const(actual: &'static str, suffix: &str) {
        // XXX: hard-code this so the crate name isn't accidentally changed.
        const CRATE_NAME: &str = "astria_sequencer_relayer";
        let expected = format!("{CRATE_NAME}_{suffix}");
        assert_eq!(expected, actual);
    }

    #[test]
    fn metrics_are_as_expected() {
        assert_const(CELESTIA_SUBMISSION_HEIGHT, "celestia_submission_height");
        assert_const(CELESTIA_SUBMISSION_COUNT, "celestia_submission_count");
        assert_const(
            CELESTIA_SUBMISSION_FAILURE_COUNT,
            "celestia_submission_failure_count",
        );
        assert_const(BLOCKS_PER_CELESTIA_TX, "blocks_per_celestia_tx");
        assert_const(BLOBS_PER_CELESTIA_TX, "blobs_per_celestia_tx");
        assert_const(BYTES_PER_CELESTIA_TX, "bytes_per_celestia_tx");
        assert_const(
            CELESTIA_PAYLOAD_CREATION_LATENCY,
            "celestia_payload_creation_latency",
        );
        assert_const(CELESTIA_SUBMISSION_LATENCY, "celestia_submission_latency");
        assert_const(
            SEQUENCER_BLOCK_FETCH_FAILURE_COUNT,
            "sequencer_block_fetch_failure_count",
        );
        assert_const(
            SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT,
            "sequencer_height_fetch_failure_count",
        );
        assert_const(SEQUENCER_SUBMISSION_HEIGHT, "sequencer_submission_height");
        assert_const(
            COMPRESSION_RATIO_FOR_ASTRIA_BLOCK,
            "compression_ratio_for_astria_block",
        );
    }
}
