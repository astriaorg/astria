//! The blobs of data that are are submitted to celestia.

use celestia_types::nmt::{
    Namespace,
    NS_ID_V0_SIZE,
};
use sequencer_types::ChainId;
use serde::{
    Deserialize,
    Serialize,
};
use sha2::{
    Digest as _,
    Sha256,
};
use tendermint::{
    block::Header,
    Hash,
};

/// Utility to create a v0 celestia namespace from the sha256 of `bytes`.
#[must_use]
#[allow(clippy::missing_panics_doc)] // OK because this is checked with a const assertion
pub fn celestia_namespace_v0_from_hashed_bytes(bytes: &[u8]) -> Namespace {
    // ensure that the conversion to `id` does not fail.
    // clippy: `NS_ID_V0_SIZE` is imported from a foreign crate. Catches
    // breaking changes.
    #[allow(clippy::assertions_on_constants)]
    const _: () = assert!(NS_ID_V0_SIZE < 32);
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let id = <[u8; NS_ID_V0_SIZE]>::try_from(&result[0..NS_ID_V0_SIZE])
        .expect("must not fail as hash is always 32 bytes and NS_ID_V0_SIZE < 32");
    Namespace::const_v0(id)
}

/// Data that is serialized and submitted to celestia as a blob under the sequencer namespace.
///
/// It contains all the other chain IDs (and thus, namespaces) that were also written to in the same
/// block.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SequencerNamespaceData {
    pub block_hash: Hash,
    pub header: Header,
    pub rollup_chain_ids: Vec<ChainId>,
    pub action_tree_root: [u8; 32],
    pub action_tree_root_inclusion_proof: merkle::Proof,
    pub chain_ids_commitment: [u8; 32],
    pub chain_ids_commitment_inclusion_proof: merkle::Proof,
}

#[derive(Debug, thiserror::Error)]
#[error(
    "failed to verify the rollup transactions and inclusion proof contained in the celestia blob \
     against the provided root hash"
)]
pub struct RollupVerificationFailure {
    _private: (),
}

/// Data that is serialized and submitted to celestia as a blob under rollup-specific namespaces.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RollupNamespaceData {
    pub block_hash: Hash,
    pub chain_id: ChainId,
    pub rollup_txs: Vec<Vec<u8>>,
    pub inclusion_proof: merkle::Proof,
}

impl RollupNamespaceData {
    /// Verifies `self.inclusion_proof` given the chain ID and the root of the merkle tree
    /// constructed from `self.rollup_txs` and the provided `root_hash`.
    ///
    /// # Errors
    /// Returns an error if the inclusion proof could not be verified.
    pub fn verify_inclusion_proof(
        &self,
        root_hash: [u8; 32],
    ) -> Result<(), RollupVerificationFailure> {
        let rollup_data_root = merkle::Tree::from_leaves(&self.rollup_txs).root();
        if !self
            .inclusion_proof
            .audit()
            .with_root(root_hash)
            .with_leaf_builder()
            .write(self.chain_id.as_ref())
            .write(&rollup_data_root)
            .finish_leaf()
            .perform()
        {
            return Err(RollupVerificationFailure {
                _private: (),
            });
        }
        Ok(())
    }
}
