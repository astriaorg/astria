use astria_core::{
    brotli::decompress_bytes,
    generated::sequencerblock::v1alpha1::{
        SubmittedMetadataList,
        SubmittedRollupDataList,
    },
    sequencerblock::v1alpha1::{
        celestia::{
            SubmittedMetadataError,
            SubmittedRollupDataError,
        },
        SubmittedMetadata,
        SubmittedRollupData,
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
            if let Some(header_list) = convert_blob_to_header_list(&blob) {
                converted_blobs.extend_from_header_list_if_well_formed(header_list);
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
            if let Some(rollup_list) = convert_blob_to_rollup_data_list(&blob) {
                converted_blobs.extend_from_rollup_data_list_if_well_formed(rollup_list);
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

/// An unsorted [`SubmittedMetadata`] and [`SubmittedRollupData`].
pub(super) struct ConvertedBlobs {
    celestia_height: u64,
    metadata: Vec<SubmittedMetadata>,
    rollup_data: Vec<SubmittedRollupData>,
}

impl ConvertedBlobs {
    pub(super) fn len_headers(&self) -> usize {
        self.metadata.len()
    }

    pub(super) fn len_rollup_data_entries(&self) -> usize {
        self.rollup_data.len()
    }

    pub(super) fn into_parts(self) -> (u64, Vec<SubmittedMetadata>, Vec<SubmittedRollupData>) {
        (self.celestia_height, self.metadata, self.rollup_data)
    }

    fn new(celestia_height: u64) -> Self {
        Self {
            celestia_height,
            metadata: Vec::new(),
            rollup_data: Vec::new(),
        }
    }

    fn push_header(&mut self, header: SubmittedMetadata) {
        self.metadata.push(header);
    }

    fn push_rollup_data(&mut self, rollup: SubmittedRollupData) {
        self.rollup_data.push(rollup);
    }

    fn extend_from_header_list_if_well_formed(&mut self, list: SubmittedMetadataList) {
        let initial_len = self.metadata.len();
        if let Err(err) = list.entries.into_iter().try_for_each(|raw| {
            let header = SubmittedMetadata::try_from_raw(raw)?;
            self.push_header(header);
            Ok::<(), SubmittedMetadataError>(())
        }) {
            info!(
                error = &err as &StdError,
                "one header in {} was not well-formed; dropping all",
                SubmittedMetadataList::full_name(),
            );
            self.metadata.truncate(initial_len);
        }
    }

    fn extend_from_rollup_data_list_if_well_formed(&mut self, list: SubmittedRollupDataList) {
        let initial_len = self.rollup_data.len();
        if let Err(err) = list.entries.into_iter().try_for_each(|raw| {
            let entry = SubmittedRollupData::try_from_raw(raw)?;
            self.push_rollup_data(entry);
            Ok::<(), SubmittedRollupDataError>(())
        }) {
            info!(
                error = &err as &StdError,
                "one entry in {} was not well-formed; dropping all",
                SubmittedRollupDataList::full_name(),
            );
            self.rollup_data.truncate(initial_len);
        }
    }
}

fn convert_blob_to_header_list(blob: &Blob) -> Option<SubmittedMetadataList> {
    let data = decompress_bytes(&blob.data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                "failed decompressing blob data; dropping the blob",
            );
        })
        .ok()?;
    let raw = SubmittedMetadataList::decode(&*data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                target = SubmittedMetadataList::full_name(),
                "failed decoding blob bytes; dropping the blob",
            );
        })
        .ok()?;
    Some(raw)
}

fn convert_blob_to_rollup_data_list(blob: &Blob) -> Option<SubmittedRollupDataList> {
    let data = decompress_bytes(&blob.data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                "failed decompressing rollup blob data; dropping the blob",
            );
        })
        .ok()?;
    let raw = SubmittedRollupDataList::decode(&*data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                target = SubmittedRollupDataList::full_name(),
                "failed decoding blob bytes; dropping the blob",
            );
        })
        .ok()?;
    Some(raw)
}
