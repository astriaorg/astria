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

/// Errors that can occur when constructing a [`SequencerNamespaceData`] from a
/// [`RawSequencerNamespaceData`].
#[derive(Debug, thiserror::Error)]
pub enum SequencerNamespaceDataConstruction {
    #[error(
        "failed to verify data hash in cometbft header against inclusion proof and action tree \
         root in sequencer block body"
    )]
    ActionTreeRootVerification,
    #[error(
        "the provided chain IDs root could not be verified against the provided proof and data \
         hash/root"
    )]
    ChainIdsRootNotInData,
    #[error(
        "the provided chain IDs root does not match the root reconstructed from the provided \
         chain IDs"
    )]
    ChainIdsRootDoesNotMatch,
    #[error(
        "block hash calculated from header does not match block hash stored in raw sequencer block"
    )]
    HashOfHeaderBlockHashMismatach,
    #[error("data hash in header not set")]
    MissingDataHash,
}

impl From<sequencer_types::sequencer_block_data::ChainIdsVerificationFailure>
    for SequencerNamespaceDataConstruction
{
    fn from(value: sequencer_types::sequencer_block_data::ChainIdsVerificationFailure) -> Self {
        use sequencer_types::sequencer_block_data::ChainIdsVerificationFailure;
        match value {
            ChainIdsVerificationFailure::ChainIdsRootNotInData => Self::ChainIdsRootNotInData,
            ChainIdsVerificationFailure::ChainIdsRootDoesNotMatch => Self::ChainIdsRootDoesNotMatch,
        }
    }
}

/// Data that is serialized and submitted to celestia as a blob under the sequencer namespace.
///
/// It contains all the other chain IDs (and thus, namespaces) that were also written to in the same
/// block.
///
/// # Invariants
/// With `SND` short for `SequencerNamespaceData`:
/// 1. `header.data_hash` is guaranteed to be set and contains 32 bytes
/// 2. `block_hash` matches the result of `SND.header.hash()`
/// 3. `action_tree_root` and `action_tree_root_inclusion_proof` must verify against
///    `header.data_hash`
/// 4. The Merkle Tree Hash built from `SND.rollup_chain_ids` must match `SND.chain_ids_commitment`
/// 5. The `SND.chain_ids_commitment` and `SND.chain_ids_commitment_proof` must verifiy against
///    `SND.data_hash`
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(try_from = "RawSequencerNamespaceData")]
#[serde(into = "RawSequencerNamespaceData")]
pub struct SequencerNamespaceData {
    block_hash: Hash,
    header: Header,
    rollup_chain_ids: Vec<ChainId>,
    action_tree_root: [u8; 32],
    action_tree_root_inclusion_proof: merkle::Proof,
    chain_ids_commitment: [u8; 32],
    chain_ids_commitment_inclusion_proof: merkle::Proof,
}

impl SequencerNamespaceData {
    /// Construct a [`SequencerNamespaceData`] from a [`RawSequencerNamespaceData`].
    ///
    /// This is [`SequencerNamespaceData`]'s only constructor to enforce its invariants.
    ///
    /// # Errors
    /// The errors cases are described by the variants of the [`SequencerNamespaceDataConstruction`]
    /// error enum.
    pub fn try_from_raw(
        raw: RawSequencerNamespaceData,
    ) -> Result<Self, SequencerNamespaceDataConstruction> {
        let RawSequencerNamespaceData {
            block_hash,
            header,
            rollup_chain_ids,
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment,
            chain_ids_commitment_inclusion_proof,
        } = raw;

        let Some(Hash::Sha256(data_hash)) = header.data_hash else {
            // header.data_hash is Option<Hash> and Hash itself has
            // variants Sha256([u8; 32]) or None.
            return Err(SequencerNamespaceDataConstruction::MissingDataHash);
        };

        let calculated_block_hash = header.hash();
        if block_hash != calculated_block_hash {
            return Err(SequencerNamespaceDataConstruction::HashOfHeaderBlockHashMismatach);
        }

        let action_tree_root_hash = sha2::Sha256::digest(action_tree_root);
        if !action_tree_root_inclusion_proof.verify(&action_tree_root_hash, data_hash) {
            return Err(SequencerNamespaceDataConstruction::ActionTreeRootVerification);
        }

        sequencer_types::sequencer_block_data::assert_chain_ids_are_included(
            chain_ids_commitment,
            &chain_ids_commitment_inclusion_proof,
            &rollup_chain_ids,
            data_hash,
        )
        .map_err(Into::<SequencerNamespaceDataConstruction>::into)?;

        Ok(Self {
            block_hash,
            header,
            rollup_chain_ids,
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment,
            chain_ids_commitment_inclusion_proof,
        })
    }

    /// Return the data hash of the cometbft header stored in the blob.
    ///
    /// # Panics
    /// This method panics if a variant of the [`SequencerNamespaceData`] was violated.
    #[must_use]
    pub fn data_hash(&self) -> [u8; 32] {
        let Some(Hash::Sha256(data_root)) = self.header.data_hash else {
            panic!(
                "data_hash must be set; this panicking means an invariant of \
                 SequencerNamespaceData was violated. This is a bug"
            );
        };
        data_root
    }

    /// Returns the cometbft block hash stored in the blob.
    #[must_use]
    pub fn block_hash(&self) -> Hash {
        self.block_hash
    }

    /// Returns the cometbft header stored in the blob.
    #[must_use]
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Returns the list of rollup chain IDs stored in the blob.
    #[must_use]
    pub fn rollup_chain_ids(&self) -> &[ChainId] {
        &self.rollup_chain_ids
    }

    /// Returns the root of the Merkle Hash Tree constructed from the sequence actions
    /// stored in the blob.
    #[must_use]
    pub fn action_tree_root(&self) -> [u8; 32] {
        self.action_tree_root
    }

    /// Returns the proof that the action tree root was included in the cometbft sequencer block.
    ///
    /// [`Self::data_hash`] is the Merkle Tree Hash this proof (together with the sha256 hash
    /// of [`Self::action_tree_root`]) is evaluated against.
    #[must_use]
    pub fn action_tree_root_inclusion_proof(&self) -> &merkle::Proof {
        &self.action_tree_root_inclusion_proof
    }

    /// Returns the Merkle Tree Hash constructed from [`Self::rollup_chain_ids`].
    #[must_use]
    pub fn chain_ids_commitment(&self) -> [u8; 32] {
        self.chain_ids_commitment
    }

    #[allow(clippy::doc_markdown)] // Clippy doesn't like CometBFT and thinks its an item.
    /// Returns the proof that the chain IDs commitment was included in the CometBFT sequencer
    /// block.
    ///
    /// `[Self::data_hash]` is the Merkle Tree Hash this proof (together with the sh256 hash of
    /// [`Self::chain_ids_commitment`]) is evaluated against.
    #[must_use]
    pub fn chain_ids_commitment_inclusion_proof(&self) -> &merkle::Proof {
        &self.chain_ids_commitment_inclusion_proof
    }
}

/// Dumb container to serialize/deserialize a [`SequencerNamespaceData`].
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RawSequencerNamespaceData {
    pub block_hash: Hash,
    pub header: Header,
    pub rollup_chain_ids: Vec<ChainId>,
    pub action_tree_root: [u8; 32],
    pub action_tree_root_inclusion_proof: merkle::Proof,
    pub chain_ids_commitment: [u8; 32],
    pub chain_ids_commitment_inclusion_proof: merkle::Proof,
}

impl RawSequencerNamespaceData {
    /// Convert [`Self`] into a [`SequencerNamespaceData`].
    ///
    /// # Errors
    /// See [`SequencerNamespaceData::try_from_raw`] for error conditions.
    pub fn try_into_verified(
        self,
    ) -> Result<SequencerNamespaceData, SequencerNamespaceDataConstruction> {
        SequencerNamespaceData::try_from_raw(self)
    }
}

impl From<SequencerNamespaceData> for RawSequencerNamespaceData {
    fn from(data: SequencerNamespaceData) -> Self {
        let SequencerNamespaceData {
            block_hash,
            header,
            rollup_chain_ids,
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment,
            chain_ids_commitment_inclusion_proof,
        } = data;
        Self {
            block_hash,
            header,
            rollup_chain_ids,
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment,
            chain_ids_commitment_inclusion_proof,
        }
    }
}

impl TryFrom<RawSequencerNamespaceData> for SequencerNamespaceData {
    type Error = SequencerNamespaceDataConstruction;

    fn try_from(raw: RawSequencerNamespaceData) -> Result<Self, Self::Error> {
        Self::try_from_raw(raw)
    }
}

/// Information on why a rollup does not belong to a given sequencer blob.
///
/// This is retruned by the [`RollupNamespaceData::belongs_to`] method.
#[derive(Debug, thiserror::Error)]
pub enum RollupDoesNotBelong {
    #[error(
        "the block hash of the rollup blob does not match the block hash of the provided \
         sequencer blob"
    )]
    BlockHashesDoNotMatch,
    #[error("the chain ID in the rollup blobs is not listed in the provided sequencer blob")]
    ChainNotListed,
    #[error(
        "the chain ID is listed in the provided sequencer blob, but the rollup's inclusion proof \
         could not be verified against the sequencer blob's action tree root"
    )]
    RollupNotInTree,
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
    /// Utility to check if the rollup blob belongs to the given sequencer blob.
    ///
    /// Returns `Ok` If `self` belongs to `sequencer_blob`.
    ///
    /// # Errors
    /// Returns an error in the following cases:
    /// 1. the block hash of the rollup blob does not match that of `sequencer_blob`
    /// 2. `self.chain_id` is not listed in `sequencer_blob.rollup_chain_ids`
    /// 3. `self.inclusion_proof` could not be verified against `sequencer_blob.action_tree_root`.
    pub fn belongs_to(
        &self,
        sequencer_blob: &SequencerNamespaceData,
    ) -> Result<(), RollupDoesNotBelong> {
        // XXX: The return order of the error conditions is important: `RollupNotInTree`
        //      explicitly mentions that the Chain ID is listed in sequencer_blob, but
        //      that its inclusion could not be verified.
        if sequencer_blob.block_hash != self.block_hash {
            return Err(RollupDoesNotBelong::BlockHashesDoNotMatch);
        }

        if !sequencer_blob.rollup_chain_ids.contains(&self.chain_id) {
            return Err(RollupDoesNotBelong::ChainNotListed);
        }

        let rollup_data_root = merkle::Tree::from_leaves(&self.rollup_txs).root();
        if !self
            .inclusion_proof
            .audit()
            .with_root(sequencer_blob.action_tree_root)
            .with_leaf_builder()
            .write(self.chain_id.as_ref())
            .write(&rollup_data_root)
            .finish_leaf()
            .perform()
        {
            return Err(RollupDoesNotBelong::RollupNotInTree);
        }
        Ok(())
    }
}
