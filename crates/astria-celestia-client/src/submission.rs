//! Logic to convert sequencer blocks to celestia blobs before submission.

use astria_core::{
    sequencer::v1::RollupId,
    sequencerblock::v1alpha1::SequencerBlock,
};
use celestia_types::Blob;
use prost::Message as _;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ToBlobsError(ToBlobsErrorKind);

impl ToBlobsError {
    fn rollup(source: celestia_types::Error, rollup_id: RollupId) -> Self {
        Self(ToBlobsErrorKind::Rollup {
            source,
            rollup_id,
        })
    }

    fn sequencer(source: celestia_types::Error) -> Self {
        Self(ToBlobsErrorKind::Sequencer(source))
    }
}

#[derive(Debug, thiserror::Error)]
enum ToBlobsErrorKind {
    #[error(
        "failed converting sequencer block subset for rollup with ID `{rollup_id}` to Celestia \
         blob"
    )]
    Rollup {
        source: celestia_types::Error,
        rollup_id: RollupId,
    },
    #[error("failed converting sequencer block metadata to Celestia blob")]
    Sequencer(#[source] celestia_types::Error),
}

pub trait ToBlobs: Sized {
    /// Convert a sequencer block to a sequence of blobs, writing them to `blobs`.
    ///
    /// If conversion of the sequencer block fails `blobs` is left unchanged.
    ///
    /// # Errors
    ///
    /// Returns an error if conversion to a Celestia blob failed. See `[Blob::new]`
    /// for more information.
    fn try_to_blobs(self, blobs: &mut Vec<Blob>) -> Result<(), ToBlobsError>;
}

impl ToBlobs for SequencerBlock {
    fn try_to_blobs(self, blobs: &mut Vec<Blob>) -> Result<(), ToBlobsError> {
        let initial_len = blobs.len();
        if let Err(e) = convert(self, blobs) {
            blobs.truncate(initial_len);
            return Err(e);
        }
        Ok(())
    }
}

fn convert(block: SequencerBlock, blobs: &mut Vec<Blob>) -> Result<(), ToBlobsError> {
    let (sequencer_blob, rollup_blobs) = block.into_celestia_blobs();
    // Allocate extra space: one blob for the sequencer blob "header",
    // the rest for the rollup blobs.
    blobs.reserve(rollup_blobs.len() + 1);
    let sequencer_namespace =
        crate::celestia_namespace_v0_from_cometbft_str(sequencer_blob.header().chain_id().as_str());

    let header_blob = Blob::new(
        sequencer_namespace,
        sequencer_blob.into_raw().encode_to_vec(),
    )
    .map_err(ToBlobsError::sequencer)?;
    blobs.push(header_blob);
    for blob in rollup_blobs {
        let rollup_id = blob.rollup_id();
        let namespace = crate::celestia_namespace_v0_from_rollup_id(rollup_id);
        let blob = Blob::new(namespace, blob.into_raw().encode_to_vec())
            .map_err(move |source| ToBlobsError::rollup(source, rollup_id))?;
        blobs.push(blob);
    }
    Ok(())
}
