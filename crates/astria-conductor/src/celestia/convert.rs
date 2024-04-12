use astria_core::sequencer::v1::{
    CelestiaRollupBlob,
    CelestiaSequencerBlob,
};
use celestia_client::celestia_types::{
    nmt::Namespace,
    Blob,
};
use prost::{
    Message as _,
    Name as _,
};
use telemetry::display::base64;
use tracing::{
    info,
    warn,
};

use super::fetch::RawBlobs;

type StdError = dyn std::error::Error;

/// Decodes blob bytes into sequencer header or rollup items, returning
/// them grouped by their block hashes.
pub(super) fn decode_raw_blobs(
    raw_blobs: RawBlobs,
    rollup_namespace: Namespace,
    sequencer_namespace: Namespace,
) -> ConvertedBlobs {
    let mut converted_blobs = ConvertedBlobs::new(raw_blobs.celestia_height);
    for blob in raw_blobs.header_blobs {
        if blob.namespace == sequencer_namespace {
            if let Some(header) = convert_header(&blob) {
                converted_blobs.push_header(header);
            }
        } else {
            warn!(
                sequencer_namespace = %base64(sequencer_namespace.as_ref()),
                namespace_in_blob = %base64(blob.namespace.as_ref()),
                "blob's namespaces was not the expected sequencer namespace; dropping",
            );
        }
    }

    for blob in raw_blobs.rollup_blobs {
        if blob.namespace == rollup_namespace {
            if let Some(rollup) = convert_rollup(&blob) {
                converted_blobs.push_rollup(rollup);
            }
        } else {
            warn!(
                rollup_namespace = %base64(rollup_namespace.as_ref()),
                namespace_in_blob = %base64(blob.namespace.as_ref()),
                "blob's namespaces was not the expected rollup namespace; dropping",
            );
        }
    }
    converted_blobs
}

/// An unsorted [`CelestiaSequencerBlob`] and [`CelestiaRollupBlob`].
pub(super) struct ConvertedBlobs {
    celestia_height: u64,
    header_blobs: Vec<CelestiaSequencerBlob>,
    rollup_blobs: Vec<CelestiaRollupBlob>,
}

impl ConvertedBlobs {
    pub(super) fn len_header_blobs(&self) -> usize {
        self.header_blobs.len()
    }

    pub(super) fn len_rollup_blobs(&self) -> usize {
        self.rollup_blobs.len()
    }

    pub(super) fn into_parts(self) -> (u64, Vec<CelestiaSequencerBlob>, Vec<CelestiaRollupBlob>) {
        (self.celestia_height, self.header_blobs, self.rollup_blobs)
    }

    fn new(celestia_height: u64) -> Self {
        Self {
            celestia_height,
            header_blobs: Vec::new(),
            rollup_blobs: Vec::new(),
        }
    }

    fn push_header(&mut self, header: CelestiaSequencerBlob) {
        self.header_blobs.push(header);
    }

    fn push_rollup(&mut self, rollup: CelestiaRollupBlob) {
        self.rollup_blobs.push(rollup);
    }
}

fn convert_header(blob: &Blob) -> Option<CelestiaSequencerBlob> {
    use astria_core::generated::sequencer::v1::CelestiaSequencerBlob as ProtoType;
    let raw = ProtoType::decode(&*blob.data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                target = ProtoType::full_name(),
                "failed decoding blob bytes as sequencer header; dropping the blob",
            );
        })
        .ok()?;
    CelestiaSequencerBlob::try_from_raw(raw)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                "failed verifying decoded sequencer header; dropping it"
            );
        })
        .ok()
}

fn convert_rollup(blob: &Blob) -> Option<CelestiaRollupBlob> {
    use astria_core::generated::sequencer::v1::CelestiaRollupBlob as ProtoType;
    let raw_blob = ProtoType::decode(&*blob.data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                target = ProtoType::full_name(),
                "failed decoding blob bytes as rollup element; dropping the blob",
            );
        })
        .ok()?;
    CelestiaRollupBlob::try_from_raw(raw_blob)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                "failed verifying decoded rollup element; dropping it"
            );
        })
        .ok()
}
