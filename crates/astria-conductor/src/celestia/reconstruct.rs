use std::collections::HashMap;

use astria_core::{
    primitive::v1::RollupId,
    sequencerblock::v1alpha1::{
        celestia::UncheckedSubmittedMetadata,
        SubmittedMetadata,
        SubmittedRollupData,
    },
};
use telemetry::display::base64;
use tracing::{
    info,
    warn,
};

use super::{
    verify::VerifiedBlobs,
    ReconstructedBlock,
};

/// Reconstructs block information from verified blocks.
///
/// The reconstructed blocks contain a block hash, Sequencer header and rollup transactions
/// (from rollup blobs, if present).
///
/// The `verified_blobs` are guaranteed to have unique Sequencer block hashes.
///
/// This process works in the following way:
/// 1. Block execution containing rollup data: each rollup blob contains a sequencer block hash that
///    is matched against `verified_blocks`
///    - The rollup blob's rollup ID, transactions, and proof area used to reconstruct a Merkle Tree
///      Hash, which must match the root stored in the Sequencer header blob. If it does, a block is
///      reconstructed from the information stored in the header and rollup blobs. The sequencer
///      header blob is removed from the map.
///    - If no sequencer header blob matches the rollup blob (no matching block hash or the Merkle
///      path audit failing), then the rollup blob is dropped.
/// 2. Empty block execution (no rollup data): The remaining Sequencer blobs (i.e. those that had no
///    matching rollup blob) are checked for `rollup_id`:
///    - if they contained `rollup_id` they are dropped (as they should have had a matching blob but
///      none was found).
///    - if they did not contain `rollup_id` a Sequencer block is reconstructed.
pub(super) fn reconstruct_blocks_from_verified_blobs(
    verified_blobs: VerifiedBlobs,
    rollup_id: RollupId,
) -> Vec<ReconstructedBlock> {
    let (celestia_height, mut header_blobs, rollup_blobs) = verified_blobs.into_parts();

    let mut reconstructed_blocks = Vec::new();

    // match rollup blobs to header blobs
    for rollup in rollup_blobs {
        if let Some(header_blob) =
            remove_header_blob_matching_rollup_blob(&mut header_blobs, &rollup)
        {
            let UncheckedSubmittedMetadata {
                block_hash,
                header,
                ..
            } = header_blob.into_unchecked();
            reconstructed_blocks.push(ReconstructedBlock {
                celestia_height,
                block_hash,
                header,
                transactions: rollup.into_unchecked().transactions,
            });
        } else {
            let reason = if header_blobs.contains_key(rollup.sequencer_block_hash()) {
                "sequencer header blobs with the same block hash as the rollup blob found, but the \
                 rollup's Merkle proof did not lead any Merkle roots"
            } else {
                "no sequencer header blob matching the rollup blob's block hash found"
            };
            info!(
                block_hash = %base64(&rollup.sequencer_block_hash()),
                reason,
                "dropping rollup blob",
            );
        }
    }

    // check left-over header blobs if they expected a rollup blob but none could be found.
    for header_blob in header_blobs.into_values() {
        if header_blob.contains_rollup_id(rollup_id) {
            warn!(
                block_hash = %base64(header_blob.block_hash()),
                "sequencer header blob contains the target rollup ID, but no matching rollup blob was found; dropping it",
            );
        } else {
            reconstructed_blocks.push(ReconstructedBlock {
                celestia_height,
                block_hash: *header_blob.block_hash(),
                header: header_blob.into_unchecked().header,
                transactions: vec![],
            });
        }
    }
    reconstructed_blocks
}

fn remove_header_blob_matching_rollup_blob(
    headers: &mut HashMap<[u8; 32], SubmittedMetadata>,
    rollup: &SubmittedRollupData,
) -> Option<SubmittedMetadata> {
    // chaining methods and returning () to use the ? operator and to not bind the value
    headers
        .get(rollup.sequencer_block_hash())
        .and_then(|header| {
            verify_rollup_blob_against_sequencer_blob(rollup, header).then_some(())
        })?;
    headers.remove(rollup.sequencer_block_hash())
}

fn verify_rollup_blob_against_sequencer_blob(
    rollup_blob: &SubmittedRollupData,
    sequencer_blob: &SubmittedMetadata,
) -> bool {
    rollup_blob
        .proof()
        .audit()
        .with_root(*sequencer_blob.rollup_transactions_root())
        .with_leaf_builder()
        .write(rollup_blob.rollup_id().get())
        .write(&merkle::Tree::from_leaves(rollup_blob.transactions()).root())
        .finish_leaf()
        .perform()
}
