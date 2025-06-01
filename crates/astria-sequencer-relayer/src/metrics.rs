use std::time::Duration;

use telemetry::{
    metric_names,
    metrics::{
        Counter,
        Gauge,
        Histogram,
        Recorder,
        RegisteringBuilder,
    },
};

pub struct Metrics {
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
    celestia_fees_total_utia: Gauge,
    celestia_fees_utia_per_uncompressed_blob_byte: Gauge,
    celestia_fees_utia_per_compressed_blob_byte: Gauge,
}

impl Metrics {
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
        self.blocks_per_celestia_tx.record(block_count);
    }

    pub(crate) fn record_blobs_per_celestia_tx(&self, blob_count: usize) {
        self.blobs_per_celestia_tx.record(blob_count);
    }

    pub(crate) fn record_bytes_per_celestia_tx(&self, byte_count: usize) {
        self.bytes_per_celestia_tx.record(byte_count);
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

    pub(crate) fn set_celestia_fees_total_utia(&self, utia: u64) {
        self.celestia_fees_total_utia.set(utia);
    }

    pub(crate) fn set_celestia_fees_utia_per_uncompressed_blob_byte(&self, utia: f64) {
        self.celestia_fees_utia_per_uncompressed_blob_byte.set(utia);
    }

    pub(crate) fn set_celestia_fees_utia_per_compressed_blob_byte(&self, utia: f64) {
        self.celestia_fees_utia_per_compressed_blob_byte.set(utia);
    }
}

impl telemetry::Metrics for Metrics {
    type Config = ();

    #[expect(
        clippy::too_many_lines,
        reason = "this is reasonable as we have a lot of metrics to register; the function is not \
                  complex, just long"
    )]
    fn register<R: Recorder>(
        builder: &mut RegisteringBuilder<R>,
        _config: &Self::Config,
    ) -> Result<Self, telemetry::metrics::Error> {
        let celestia_submission_height = builder
            .new_counter_factory(
                CELESTIA_SUBMISSION_HEIGHT,
                "The height of the last blob successfully submitted to Celestia",
            )?
            .register()?;

        let celestia_submission_count = builder
            .new_counter_factory(
                CELESTIA_SUBMISSION_COUNT,
                "The number of calls made to submit to Celestia",
            )?
            .register()?;

        let celestia_submission_failure_count = builder
            .new_counter_factory(
                CELESTIA_SUBMISSION_FAILURE_COUNT,
                "The number of calls made to submit to Celestia which have failed",
            )?
            .register()?;

        let blocks_per_celestia_tx = builder
            .new_histogram_factory(
                BLOCKS_PER_CELESTIA_TX,
                "The number of Astria blocks per Celestia submission",
            )?
            .register()?;

        let blobs_per_celestia_tx = builder
            .new_histogram_factory(
                BLOBS_PER_CELESTIA_TX,
                "The number of blobs (Astria Sequencer blocks converted to Celestia blobs) per \
                 Celestia submission",
            )?
            .register()?;

        let bytes_per_celestia_tx = builder
            .new_histogram_factory(
                BYTES_PER_CELESTIA_TX,
                "The total number of payload bytes (Astria Sequencer blocks converted to Celestia \
                 blobs) per Celestia submission",
            )?
            .register()?;

        let celestia_payload_creation_latency = builder
            .new_histogram_factory(
                CELESTIA_PAYLOAD_CREATION_LATENCY,
                "The time it takes to create a new payload for submitting to Celestia (encoding \
                 to protobuf, compression, creating blobs)",
            )?
            .register()?;

        let celestia_submission_latency = builder
            .new_histogram_factory(
                CELESTIA_SUBMISSION_LATENCY,
                "The time it takes to submit a blob to Celestia",
            )?
            .register()?;

        let sequencer_block_fetch_failure_count = builder
            .new_counter_factory(
                SEQUENCER_BLOCK_FETCH_FAILURE_COUNT,
                "The number of calls made to fetch a block from sequencer which have failed",
            )?
            .register()?;

        let sequencer_height_fetch_failure_count = builder
            .new_counter_factory(
                SEQUENCER_HEIGHT_FETCH_FAILURE_COUNT,
                "The number of calls made to fetch the current height from sequencer which have \
                 failed",
            )?
            .register()?;

        let sequencer_submission_height = builder
            .new_counter_factory(
                SEQUENCER_SUBMISSION_HEIGHT,
                "The height of the highest sequencer block successfully submitted to Celestia",
            )?
            .register()?;

        let compression_ratio_for_astria_block = builder
            .new_gauge_factory(
                COMPRESSION_RATIO_FOR_ASTRIA_BLOCK,
                "Ratio of uncompressed:compressed data size for all `blob.data`s in an Astria \
                 block",
            )?
            .register()?;

        let celestia_fees_total_utia = builder
            .new_gauge_factory(
                CELESTIA_FEES_TOTAL_UTIA,
                "The total Celestia fees in utia for the latest successful submission",
            )?
            .register()?;

        let celestia_fees_utia_per_uncompressed_blob_byte = builder
            .new_gauge_factory(
                CELESTIA_FEES_UTIA_PER_UNCOMPRESSED_BLOB_BYTE,
                "The Celestia fees in utia per uncompressed blob byte for the latest successful \
                 submission",
            )?
            .register()?;

        let celestia_fees_utia_per_compressed_blob_byte = builder
            .new_gauge_factory(
                CELESTIA_FEES_UTIA_PER_COMPRESSED_BLOB_BYTE,
                "The Celestia fees in utia per compressed blob byte for the latest successful \
                 submission",
            )?
            .register()?;

        Ok(Self {
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
            celestia_fees_total_utia,
            celestia_fees_utia_per_uncompressed_blob_byte,
            celestia_fees_utia_per_compressed_blob_byte,
        })
    }
}

metric_names!(const METRICS_NAMES:
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
    COMPRESSION_RATIO_FOR_ASTRIA_BLOCK,
    CELESTIA_FEES_TOTAL_UTIA,
    CELESTIA_FEES_UTIA_PER_UNCOMPRESSED_BLOB_BYTE,
    CELESTIA_FEES_UTIA_PER_COMPRESSED_BLOB_BYTE
);

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_const(CELESTIA_FEES_TOTAL_UTIA, "celestia_fees_total_utia");
        assert_const(
            CELESTIA_FEES_UTIA_PER_UNCOMPRESSED_BLOB_BYTE,
            "celestia_fees_utia_per_uncompressed_blob_byte",
        );
        assert_const(
            CELESTIA_FEES_UTIA_PER_COMPRESSED_BLOB_BYTE,
            "celestia_fees_utia_per_compressed_blob_byte",
        );
    }
}
