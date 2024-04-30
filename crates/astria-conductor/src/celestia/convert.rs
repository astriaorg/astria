use astria_core::{
    brotli::decompress_bytes,
    sequencerblock::v1alpha1::{
        CelestiaHeader,
        CelestiaRollupData,
    },
};
use celestia_types::{
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
    header_blobs: Vec<CelestiaHeader>,
    rollup_blobs: Vec<CelestiaRollupData>,
}

impl ConvertedBlobs {
    pub(super) fn len_header_blobs(&self) -> usize {
        self.header_blobs.len()
    }

    pub(super) fn len_rollup_blobs(&self) -> usize {
        self.rollup_blobs.len()
    }

    pub(super) fn into_parts(self) -> (u64, Vec<CelestiaHeader>, Vec<CelestiaRollupData>) {
        (self.celestia_height, self.header_blobs, self.rollup_blobs)
    }

    fn new(celestia_height: u64) -> Self {
        Self {
            celestia_height,
            header_blobs: Vec::new(),
            rollup_blobs: Vec::new(),
        }
    }

    fn push_header(&mut self, header: CelestiaHeader) {
        self.header_blobs.push(header);
    }

    fn push_rollup(&mut self, rollup: CelestiaRollupData) {
        self.rollup_blobs.push(rollup);
    }
}

fn convert_header(blob: &Blob) -> Option<CelestiaHeader> {
    use astria_core::generated::sequencerblock::v1alpha1::CelestiaHeader as ProtoType;
    let data = decompress_bytes(&blob.data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                "failed decompressing blob data; dropping the blob",
            );
        })
        .ok()?;
    let raw = ProtoType::decode(&*data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                target = ProtoType::full_name(),
                "failed decoding blob bytes as sequencer header; dropping the blob",
            );
        })
        .ok()?;
    CelestiaHeader::try_from_raw(raw)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                "failed verifying decoded sequencer header; dropping it"
            );
        })
        .ok()
}

fn convert_rollup(blob: &Blob) -> Option<CelestiaRollupData> {
    use astria_core::generated::sequencerblock::v1alpha1::CelestiaRollupData as ProtoType;
    let data = decompress_bytes(&blob.data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                "failed decompressing rollup blob data; dropping the blob",
            );
        })
        .ok()?;
    let raw_blob = ProtoType::decode(&*data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                target = ProtoType::full_name(),
                "failed decoding blob bytes as rollup element; dropping the blob",
            );
        })
        .ok()?;
    CelestiaRollupData::try_from_raw(raw_blob)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                "failed verifying decoded rollup element; dropping it"
            );
        })
        .ok()
}
