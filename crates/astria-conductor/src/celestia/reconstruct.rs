use std::collections::HashMap;

use astria_core::sequencerblock::v1alpha1::{
    CelestiaRollupBlob,
    CelestiaSequencerBlob,
};
use astria_core::sequencer::v1::RollupId;
use sequencer_client::tendermint::block::Height as SequencerHeight;
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
/// The reconstructed blocks contain a block hash, tendermint header (both extracted from
/// sequencer header blobs), and transactions (from rollup blobs, if present).
///
/// This process works in the following way:
/// 1. Header blobs are placed in a map using a key `(block_hash, height)`.
///    - header blobs with the same `(block_hash, height)` are dropped.
/// 2. Each rollup blob is matched against all header blobs using Merkle proofs and roots.
///    - A Sequencer block is reconstructed from the the first such match.
///    - All all other sequencer header blobs with the same block hash but different height are
///      dropped (it is assumed that sequencer blobs of different height cannot have the same block
///      hash).
///    - A rollup blob that has no matching sequencer header blob is dropped.
/// 3. The remaining Sequencer blobs (i.e. those that had no matching rollup blob) are checked for
///    `rollup_id`:
///    - if they contained `rollup_id` they are dropped (as they should have had a matching blob but
///      none was found).
///    - if they did not contain `rollup_id` a Sequencer block is reconstructed.
pub(super) fn reconstruct_blocks_from_verified_blobs(
    verified_blobs: VerifiedBlobs,
    rollup_id: RollupId,
) -> Vec<ReconstructedBlock> {
    let (celestia_height, header_blobs, rollup_blobs) = verified_blobs.into_parts();

    let mut headers = BlockHashAndHeightToHeader::from_header_blobs(header_blobs);

    let mut reconstructed_blocks = Vec::new();
    for rollup in rollup_blobs {
        if let Some(header_blob) = headers.remove_header_blob_matching_rollup_blob(&rollup) {
            reconstructed_blocks.push(ReconstructedBlock {
                celestia_height,
                block_hash: header_blob.block_hash(),
                header: header_blob.into_unchecked().header,
                transactions: rollup.into_unchecked().transactions,
            });
        } else {
            let reason = if headers.any_headers_with_block_hash(rollup.sequencer_block_hash()) {
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

    let remaining_headers = headers.map;

    for header_blob in remaining_headers.into_values() {
        if header_blob.contains_rollup_id(rollup_id) {
            warn!(
                block_hash = %base64(&header_blob.block_hash()),
                "sequencer header blob contains the target rollup ID, but no matching rollup blob was found; the blob",
            );
        } else {
            reconstructed_blocks.push(ReconstructedBlock {
                celestia_height,
                block_hash: header_blob.block_hash(),
                header: header_blob.into_unchecked().header,
                transactions: vec![],
            });
        }
    }
    reconstructed_blocks
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Key {
    block_hash: [u8; 32],
    height: SequencerHeight,
}

impl Key {
    fn from_header_blob(val: &CelestiaSequencerBlob) -> Self {
        Self {
            block_hash: val.block_hash(),
            height: val.height(),
        }
    }
}

struct BlockHashAndHeightToHeader {
    // The sequencer header blobs grouped by (block_hash, sequencer_height)
    map: HashMap<Key, CelestiaSequencerBlob>,

    // A reverse index of block_hash -> [(block_hash, sequencer_height)]
    reverse_index: HashMap<[u8; 32], Vec<Key>>,
}

impl BlockHashAndHeightToHeader {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            reverse_index: HashMap::new(),
        }
    }

    fn from_header_blobs(blobs: Vec<CelestiaSequencerBlob>) -> Self {
        let mut this = Self::new();
        for blob in blobs {
            if let Some(dropped) = this.insert(blob) {
                warn!(
                    block_hash = %base64(&dropped.block_hash()),
                    sequencer_height = %dropped.height(),
                    "more than one sequencer header blob with the same block hash and height passed verification; dropping previous",
                );
            }
        }
        this
    }

    fn insert(&mut self, blob: CelestiaSequencerBlob) -> Option<CelestiaSequencerBlob> {
        let key = Key::from_header_blob(&blob);
        if let Some(old) = self.map.insert(key, blob) {
            return Some(old);
        }
        self.reverse_index
            .entry(key.block_hash)
            .and_modify(|keys| keys.push(key))
            .or_insert_with(|| vec![key]);
        None
    }

    fn any_headers_with_block_hash(&self, block_hash: [u8; 32]) -> bool {
        self.reverse_index.contains_key(&block_hash)
    }

    fn remove_header_blob_matching_rollup_blob(
        &mut self,
        rollup: &CelestiaRollupBlob,
    ) -> Option<CelestiaSequencerBlob> {
        let position_of_matching_key = self
            .reverse_index
            .get(&rollup.sequencer_block_hash())?
            .iter()
            .position(|key| {
                let header = self
                    .map
                    .get(key)
                    .expect("keys stored in the reverse index must exist in the map");
                verify_rollup_blob_against_sequencer_blob(rollup, header)
            })?;

        let mut all_keys = self
            .reverse_index
            .remove(&rollup.sequencer_block_hash())
            .expect("entry exists, it was just accessed above");

        let target_key = all_keys.swap_remove(position_of_matching_key);

        let target_header = self
            .map
            .remove(&target_key)
            .expect("keys retrieved from the reverse index must exist");

        for key in all_keys {
            self.map.remove(&key);
        }

        Some(target_header)
    }
}

fn verify_rollup_blob_against_sequencer_blob(
    rollup_blob: &CelestiaRollupBlob,
    sequencer_blob: &CelestiaSequencerBlob,
) -> bool {
    rollup_blob
        .proof()
        .audit()
        .with_root(sequencer_blob.header().rollup_transactions_root())
        .with_leaf_builder()
        .write(&rollup_blob.rollup_id().get())
        .write(&merkle::Tree::from_leaves(rollup_blob.transactions()).root())
        .finish_leaf()
        .perform()
}
