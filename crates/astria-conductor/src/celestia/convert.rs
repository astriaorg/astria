use astria_core::{
    brotli::decompress_bytes,
    generated::sequencerblock::v1alpha1::{
        CelestiaHeaderList,
        CelestiaRollupDataList,
    },
    sequencerblock::v1alpha1::{
        celestia::{
            CelestiaRollupBlobError,
            CelestiaSequencerBlobError,
        },
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

/// An unsorted [`CelestiaSequencerBlob`] and [`CelestiaRollupBlob`].
pub(super) struct ConvertedBlobs {
    celestia_height: u64,
    headers: Vec<CelestiaHeader>,
    rollup_data_entries: Vec<CelestiaRollupData>,
}

impl ConvertedBlobs {
    pub(super) fn len_headers(&self) -> usize {
        self.headers.len()
    }

    pub(super) fn len_rollup_data_entries(&self) -> usize {
        self.rollup_data_entries.len()
    }

    pub(super) fn into_parts(self) -> (u64, Vec<CelestiaHeader>, Vec<CelestiaRollupData>) {
        (self.celestia_height, self.headers, self.rollup_data_entries)
    }

    fn new(celestia_height: u64) -> Self {
        Self {
            celestia_height,
            headers: Vec::new(),
            rollup_data_entries: Vec::new(),
        }
    }

    fn push_header(&mut self, header: CelestiaHeader) {
        self.headers.push(header);
    }

    fn push_rollup_data(&mut self, rollup: CelestiaRollupData) {
        self.rollup_data_entries.push(rollup);
    }

    fn extend_from_header_list_if_well_formed(&mut self, list: CelestiaHeaderList) {
        let initial_len = self.headers.len();
        if let Err(err) = list.headers.into_iter().try_for_each(|raw| {
            let header = CelestiaHeader::try_from_raw(raw)?;
            self.push_header(header);
            Ok::<(), CelestiaSequencerBlobError>(())
        }) {
            info!(
                error = &err as &StdError,
                "one header in {} was not well-formed; dropping all",
                CelestiaHeaderList::full_name(),
            );
            self.headers.truncate(initial_len);
        }
    }

    fn extend_from_rollup_data_list_if_well_formed(&mut self, list: CelestiaRollupDataList) {
        let initial_len = self.rollup_data_entries.len();
        if let Err(err) = list.entries.into_iter().try_for_each(|raw| {
            let entry = CelestiaRollupData::try_from_raw(raw)?;
            self.push_rollup_data(entry);
            Ok::<(), CelestiaRollupBlobError>(())
        }) {
            info!(
                error = &err as &StdError,
                "one entry in {} was not well-formed; dropping all",
                CelestiaRollupDataList::full_name(),
            );
            self.rollup_data_entries.truncate(initial_len);
        }
    }
}

fn convert_blob_to_header_list(blob: &Blob) -> Option<CelestiaHeaderList> {
    let data = decompress_bytes(&blob.data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                "failed decompressing blob data; dropping the blob",
            );
        })
        .ok()?;
    let raw = CelestiaHeaderList::decode(&*data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                target = CelestiaHeaderList::full_name(),
                "failed decoding blob bytes; dropping the blob",
            );
        })
        .ok()?;
    Some(raw)
}

fn convert_blob_to_rollup_data_list(blob: &Blob) -> Option<CelestiaRollupDataList> {
    let data = decompress_bytes(&blob.data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                "failed decompressing rollup blob data; dropping the blob",
            );
        })
        .ok()?;
    let raw = CelestiaRollupDataList::decode(&*data)
        .inspect_err(|err| {
            info!(
                error = err as &StdError,
                target = CelestiaRollupDataList::full_name(),
                "failed decoding blob bytes; dropping the blob",
            );
        })
        .ok()?;
    Some(raw)
}
