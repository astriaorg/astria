use astria_core::{
    brotli::compress_bytes,
    primitive::v1::RollupId,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use celestia_client::celestia_types::{
    nmt::Namespace,
    Blob,
};
use prost::Message as _;
use sequencer_client::SequencerBlock;
use tendermint::block::Height as SequencerHeight;
use tracing::debug;

use crate::metrics_init;

// allow: the signature is dictated by the `serde(serialize_with = ...)` attribute.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn serialize_height<S>(height: &SequencerHeight, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    serializer.serialize_u64(height.value())
}

fn serialize_namespace<S>(namespace: &Namespace, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    use serde::ser::Serialize as _;
    telemetry::display::base64(namespace.as_bytes()).serialize(serializer)
}

#[derive(Debug, serde::Serialize)]
pub(super) struct RollupInfo {
    number_of_transactions: usize,
    #[serde(serialize_with = "serialize_namespace")]
    celestia_namespace: Namespace,
    sequencer_rollup_id: RollupId,
}

/// Information about the a block that was converted to blobs.
#[derive(Debug, serde::Serialize)]
pub(super) struct ConversionInfo {
    #[serde(serialize_with = "serialize_height")]
    pub(super) sequencer_height: SequencerHeight,
    #[serde(serialize_with = "serialize_namespace")]
    pub(super) sequencer_namespace: Namespace,
    pub(super) rollups: Vec<RollupInfo>,
}

/// The result of a block that was converted to blobs.
pub(super) struct Converted {
    pub(super) blobs: Vec<Blob>,
    pub(super) info: ConversionInfo,
}

pub(super) fn convert(block: SequencerBlock) -> eyre::Result<Converted> {
    let sequencer_height = block.height();
    let mut total_data_uncompressed_size = 0;
    let mut total_data_compressed_size = 0;

    let (sequencer_blob, rollup_blobs) = block.into_celestia_blobs();
    // Allocate extra space: one blob for the sequencer blob "header",
    // the rest for the rollup blobs.
    let mut blobs = Vec::with_capacity(rollup_blobs.len() + 1);
    let sequencer_namespace = celestia_client::celestia_namespace_v0_from_str(
        sequencer_blob.header().chain_id().as_str(),
    );
    let sequencer_blob_raw = sequencer_blob.into_raw().encode_to_vec();
    total_data_uncompressed_size += sequencer_blob_raw.len();
    let compressed_sequencer_blob_raw =
        compress_bytes(&sequencer_blob_raw).wrap_err("failed compressing sequencer blob")?;
    total_data_compressed_size += compressed_sequencer_blob_raw.len();

    let header_blob = Blob::new(sequencer_namespace, compressed_sequencer_blob_raw)
        .wrap_err("failed creating head Celestia blob")?;
    blobs.push(header_blob);
    let mut rollups = Vec::new();
    for blob in rollup_blobs {
        let rollup_id = blob.rollup_id();
        let namespace = celestia_client::celestia_namespace_v0_from_rollup_id(rollup_id);
        let info = RollupInfo {
            number_of_transactions: blob.transactions().len(),
            celestia_namespace: namespace,
            sequencer_rollup_id: blob.rollup_id(),
        };
        let raw_blob = blob.into_raw().encode_to_vec();
        total_data_uncompressed_size += raw_blob.len();
        let compressed_blob = compress_bytes(&raw_blob)
            .wrap_err_with(|| format!("failed compressing rollup `{rollup_id}`"))?;
        total_data_compressed_size += compressed_blob.len();
        let blob = Blob::new(namespace, compressed_blob)
            .wrap_err_with(|| format!("failed creating blob for rollup `{rollup_id}`"))?;
        blobs.push(blob);
        rollups.push(info);
    }

    // gauges require f64, it's okay if the metrics get messed up by overflow or precision loss
    #[allow(clippy::cast_precision_loss)]
    let compression_ratio = total_data_uncompressed_size as f64 / total_data_compressed_size as f64;
    debug!(
        sequencer_height = %sequencer_height,
        total_data_compressed_size = total_data_compressed_size,
        compression_ratio = compression_ratio,
        "converted blocks into blobs with compressed data",
    );
    #[allow(clippy::cast_precision_loss)]
    metrics::gauge!(metrics_init::TOTAL_BLOB_DATA_SIZE_FOR_ASTRIA_BLOCK)
        .set(total_data_compressed_size as f64);
    metrics::gauge!(metrics_init::COMPRESSION_RATIO_FOR_ASTRIA_BLOCK).set(compression_ratio);

    Ok(Converted {
        blobs,
        info: ConversionInfo {
            sequencer_height,
            sequencer_namespace,
            rollups,
        },
    })
}
