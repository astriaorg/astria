use std::collections::HashSet;

use astria_core::primitive::v1::RollupId;
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

/// Information about a sequencer block that was converted to blobs.
#[derive(Debug, serde::Serialize)]
pub(super) struct ConversionInfo {
    #[serde(serialize_with = "serialize_height")]
    pub(super) sequencer_height: SequencerHeight,
    #[serde(serialize_with = "serialize_namespace")]
    pub(super) sequencer_namespace: Namespace,
    pub(super) rollups_included: Vec<RollupInfo>,
    pub(super) rollups_excluded: Vec<RollupInfo>,
}

/// The result of a sequencer block that was converted to blobs.
pub(super) struct Converted {
    pub(super) blobs: Vec<Blob>,
    pub(super) info: ConversionInfo,
}

/// Convert the given sequencer block into a collection of blobs and related metadata.
///
/// If `include` is empty, blobs from all rollups will be included.  Otherwise only blobs from the
/// rollups specified in `include` will be included.
// allow: we'd need static lifetime on a ref to avoid pass-by-value here.
#[allow(clippy::needless_pass_by_value)]
pub(super) fn convert(
    block: SequencerBlock,
    include: HashSet<RollupId>,
) -> eyre::Result<Converted> {
    let sequencer_height = block.height();

    let (sequencer_blob, rollup_blobs) = block.into_celestia_blobs();
    // Allocate extra space: one blob for the sequencer blob "header",
    // the rest for the rollup blobs.
    let mut blobs = Vec::with_capacity(rollup_blobs.len() + 1);
    let sequencer_namespace = celestia_client::celestia_namespace_v0_from_str(
        sequencer_blob.header().chain_id().as_str(),
    );

    let header_blob = Blob::new(
        sequencer_namespace,
        sequencer_blob.into_raw().encode_to_vec(),
    )
    .wrap_err("failed creating head Celestia blob")?;
    blobs.push(header_blob);
    let mut rollups_included = Vec::new();
    let mut rollups_excluded = Vec::new();
    for blob in rollup_blobs {
        let rollup_id = blob.rollup_id();
        let namespace = celestia_client::celestia_namespace_v0_from_rollup_id(rollup_id);
        let info = RollupInfo {
            number_of_transactions: blob.transactions().len(),
            celestia_namespace: namespace,
            sequencer_rollup_id: rollup_id,
        };
        if include.is_empty() || include.contains(&rollup_id) {
            let blob = Blob::new(namespace, blob.into_raw().encode_to_vec())
                .wrap_err_with(|| format!("failed creating blob for rollup `{rollup_id}`"))?;
            blobs.push(blob);
            rollups_included.push(info);
        } else {
            rollups_excluded.push(info);
        }
    }
    Ok(Converted {
        blobs,
        info: ConversionInfo {
            sequencer_height,
            sequencer_namespace,
            rollups_included,
            rollups_excluded,
        },
    })
}
