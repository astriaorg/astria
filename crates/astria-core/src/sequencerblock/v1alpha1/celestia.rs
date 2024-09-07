use bytes::Bytes;
use sha2::{
    Digest as _,
    Sha256,
};

use super::{
    block::{
        RollupTransactionsParts,
        SequencerBlock,
        SequencerBlockHeader,
        SequencerBlockHeaderError,
    },
    raw,
    IncorrectRollupIdLength,
    RollupId,
};
use crate::Protobuf;

/// A [`super::SequencerBlock`] split and prepared for submission to a data availability provider.
///
/// Consists of a head [`SubmittedMetadata`] and a tail of [`SubmittedRollupData`]s.
pub(super) struct PreparedBlock {
    head: SubmittedMetadata,
    tail: Vec<SubmittedRollupData>,
}

impl PreparedBlock {
    /// Construct a bundle of celestia blobs from a [`super::SequencerBlock`].
    #[must_use]
    pub(super) fn from_sequencer_block(block: SequencerBlock) -> Self {
        let super::block::SequencerBlockParts {
            block_hash,
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = block.into_parts();

        let head = SubmittedMetadata {
            block_hash,
            header,
            rollup_ids: rollup_transactions.keys().copied().collect(),
            rollup_transactions_proof,
            rollup_ids_proof,
        };

        let mut tail = Vec::with_capacity(rollup_transactions.len());
        for (rollup_id, rollup_txs) in rollup_transactions {
            let RollupTransactionsParts {
                transactions,
                proof,
                ..
            } = rollup_txs.into_parts();
            let transactions = transactions.into_iter().map(Bytes::into).collect();
            tail.push(SubmittedRollupData {
                sequencer_block_hash: block_hash,
                rollup_id,
                transactions,
                proof,
            });
        }
        Self {
            head,
            tail,
        }
    }

    /// Returns the head and the tail of the split block, consuming it.
    pub(super) fn into_parts(self) -> (SubmittedMetadata, Vec<SubmittedRollupData>) {
        (self.head, self.tail)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed constructing a celestia rollup blob")]
#[allow(clippy::module_name_repetitions)]
pub struct SubmittedRollupDataError {
    #[source]
    kind: SubmittedRollupDataErrorKind,
}

impl SubmittedRollupDataError {
    fn field_not_set(field: &'static str) -> Self {
        Self {
            kind: SubmittedRollupDataErrorKind::FieldNotSet {
                field,
            },
        }
    }

    fn rollup_id(source: IncorrectRollupIdLength) -> Self {
        Self {
            kind: SubmittedRollupDataErrorKind::RollupId {
                source,
            },
        }
    }

    fn proof(source: <merkle::Proof as Protobuf>::Error) -> Self {
        Self {
            kind: SubmittedRollupDataErrorKind::Proof {
                source,
            },
        }
    }

    fn sequencer_block_hash(actual_len: usize) -> Self {
        Self {
            kind: SubmittedRollupDataErrorKind::SequencerBlockHash(actual_len),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum SubmittedRollupDataErrorKind {
    #[error("the expected field in the raw source type was not set: `{field}`")]
    FieldNotSet { field: &'static str },
    #[error("failed converting the provided bytes to Rollup ID")]
    RollupId { source: IncorrectRollupIdLength },
    #[error("failed constructing a Merkle Hash Tree Proof from the provided raw protobuf type")]
    Proof {
        source: <merkle::Proof as Protobuf>::Error,
    },
    #[error(
        "the provided bytes were too short for a sequencer block hash. Expected: 32 bytes, \
         provided: {0}"
    )]
    SequencerBlockHash(usize),
}

/// A shadow of [`SubmittedRollupData`] with public access to all its fields.
///
/// At the moment there are no invariants upheld by [`SubmittedRollupData`] so
/// they can be converted directly into one another. This can change in the future.
pub struct UncheckedSubmittedRollupData {
    /// The hash of the sequencer block. Must be 32 bytes.
    pub sequencer_block_hash: [u8; 32],
    /// The 32 bytes identifying the rollup this blob belongs to. Matches
    /// `astria.sequencerblock.v1alpha1.RollupTransactions.rollup_id`
    pub rollup_id: RollupId,
    /// A list of opaque bytes that are serialized rollup transactions.
    pub transactions: Vec<Bytes>,
    /// The proof that these rollup transactions are included in sequencer block.
    pub proof: merkle::Proof,
}

impl UncheckedSubmittedRollupData {
    #[must_use]
    pub fn into_celestia_rollup_blob(self) -> SubmittedRollupData {
        SubmittedRollupData::from_unchecked(self)
    }
}

#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct SubmittedRollupData {
    /// The hash of the sequencer block. Must be 32 bytes.
    sequencer_block_hash: [u8; 32],
    /// The 32 bytes identifying the rollup this blob belongs to. Matches
    /// `astria.sequencerblock.v1alpha1.RollupTransactions.rollup_id`
    rollup_id: RollupId,
    /// A list of opaque bytes that are serialized rollup transactions.
    transactions: Vec<Bytes>,
    /// The proof that these rollup transactions are included in sequencer block.
    proof: merkle::Proof,
}

impl SubmittedRollupData {
    #[must_use]
    pub fn proof(&self) -> &merkle::Proof {
        &self.proof
    }

    #[must_use]
    pub fn transactions(&self) -> &[Bytes] {
        &self.transactions
    }

    #[must_use]
    pub fn rollup_id(&self) -> RollupId {
        self.rollup_id
    }

    #[must_use]
    pub fn sequencer_block_hash(&self) -> &[u8; 32] {
        &self.sequencer_block_hash
    }

    /// Converts from the unchecked representation of this type (its shadow).
    ///
    /// This type does not uphold any extra invariants so there are no extra checks necessary.
    #[must_use]
    pub fn from_unchecked(unchecked: UncheckedSubmittedRollupData) -> Self {
        let UncheckedSubmittedRollupData {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        } = unchecked;
        Self {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        }
    }

    /// Converts to the unchecked representation of this type (its shadow).
    ///
    /// Useful to get public access to the type's fields.
    #[must_use]
    pub fn into_unchecked(self) -> UncheckedSubmittedRollupData {
        let Self {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        } = self;
        UncheckedSubmittedRollupData {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        }
    }

    /// Converts to the raw decoded protobuf representation of this type.
    ///
    /// Useful for then encoding it as protobuf.
    #[must_use]
    pub fn into_raw(self) -> raw::SubmittedRollupData {
        let Self {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        } = self;
        raw::SubmittedRollupData {
            sequencer_block_hash: Bytes::copy_from_slice(&sequencer_block_hash),
            rollup_id: Some(rollup_id.to_raw()),
            transactions,
            proof: Some(proof.into_raw()),
        }
    }

    /// Converts from the raw decoded protobuf representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(raw: raw::SubmittedRollupData) -> Result<Self, SubmittedRollupDataError> {
        let raw::SubmittedRollupData {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        } = raw;
        let Some(rollup_id) = rollup_id else {
            return Err(SubmittedRollupDataError::field_not_set("rollup_id"));
        };
        let rollup_id =
            RollupId::try_from_raw(&rollup_id).map_err(SubmittedRollupDataError::rollup_id)?;
        let sequencer_block_hash = sequencer_block_hash.as_ref().try_into().map_err(|_| {
            SubmittedRollupDataError::sequencer_block_hash(sequencer_block_hash.len())
        })?;
        let proof = 'proof: {
            let Some(proof) = proof else {
                break 'proof Err(SubmittedRollupDataError::field_not_set("proof"));
            };
            merkle::Proof::try_from_raw(proof).map_err(SubmittedRollupDataError::proof)
        }?;
        Ok(Self {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed constructing a celestia sequencer blob")]
#[allow(clippy::module_name_repetitions)]
pub struct SubmittedMetadataError {
    #[source]
    kind: SubmittedMetadataErrorKind,
}

impl SubmittedMetadataError {
    fn block_hash(actual_len: usize) -> Self {
        Self {
            kind: SubmittedMetadataErrorKind::BlockHash(actual_len),
        }
    }

    fn header(source: SequencerBlockHeaderError) -> Self {
        Self {
            kind: SubmittedMetadataErrorKind::Header {
                source,
            },
        }
    }

    fn field_not_set(field: &'static str) -> Self {
        Self {
            kind: SubmittedMetadataErrorKind::FieldNotSet(field),
        }
    }

    fn rollup_ids(source: IncorrectRollupIdLength) -> Self {
        Self {
            kind: SubmittedMetadataErrorKind::RollupIds {
                source,
            },
        }
    }

    fn rollup_transactions_proof(source: <merkle::Proof as Protobuf>::Error) -> Self {
        Self {
            kind: SubmittedMetadataErrorKind::RollupTransactionsProof {
                source,
            },
        }
    }

    fn rollup_ids_proof(source: <merkle::Proof as Protobuf>::Error) -> Self {
        Self {
            kind: SubmittedMetadataErrorKind::RollupIdsProof {
                source,
            },
        }
    }

    fn rollup_transactions_not_in_cometbft_block() -> Self {
        Self {
            kind: SubmittedMetadataErrorKind::RollupTransactionsNotInCometBftBlock,
        }
    }

    fn rollup_ids_not_in_cometbft_block() -> Self {
        Self {
            kind: SubmittedMetadataErrorKind::RollupIdsNotInCometBftBlock,
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum SubmittedMetadataErrorKind {
    #[error(
        "the provided bytes were too short for a block hash; expected: 32 bytes, actual: {0} bytes"
    )]
    BlockHash(usize),
    #[error("failed constructing the sequencer block header from its raw source value")]
    Header { source: SequencerBlockHeaderError },
    #[error("the field of the raw source value was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("one of the rollup IDs in the raw source value was invalid")]
    RollupIds { source: IncorrectRollupIdLength },
    #[error(
        "failed constructing a Merkle Hash Tree Proof for the rollup transactions from the raw \
         raw source type"
    )]
    RollupTransactionsProof {
        source: <merkle::Proof as Protobuf>::Error,
    },
    #[error(
        "failed constructing a Merkle Hash Tree Proof for the rollup IDs from the raw raw source \
         type"
    )]
    RollupIdsProof {
        source: <merkle::Proof as Protobuf>::Error,
    },
    #[error(
        "the Merkle Tree Hash of the rollup transactions was not a leaf in the sequencer block \
         data"
    )]
    RollupTransactionsNotInCometBftBlock,
    #[error("the Merkle Tree Hash of the rollup IDs was not a leaf in the sequencer block data")]
    RollupIdsNotInCometBftBlock,
}

/// A shadow of [`SubmittedMetadata`] with public access to its fields.
///
/// This type does not guarantee any invariants and is mainly useful to get
/// access the sequencer block's internal types.
#[derive(Clone, Debug)]
pub struct UncheckedSubmittedMetadata {
    pub block_hash: [u8; 32],
    /// The original `CometBFT` header that is the input to this blob's original sequencer block.
    /// Corresponds to `astria.SequencerBlock.header`.
    pub header: SequencerBlockHeader,
    /// The rollup rollup IDs for which `SubmittedRollupData`s were submitted to celestia.
    /// Corresponds to the `astria.sequencer.v1.RollupTransactions.id` field
    /// and is extracted from `astria.SequencerBlock.rollup_transactions`.
    pub rollup_ids: Vec<RollupId>,
    /// The proof that the rollup transactions are included in sequencer block.
    /// Corresponds to `astria.SequencerBlock.rollup_transactions_proof`.
    pub rollup_transactions_proof: merkle::Proof,
    /// The proof that this sequencer blob includes all rollup IDs of the original sequencer
    /// block it was derived from. This proof together with `Sha256(MTH(rollup_ids))` (Sha256
    /// applied to the Merkle Tree Hash of the rollup ID sequence) must be equal to
    /// `header.data_hash` which itself must match
    /// `astria.SequencerBlock.header.data_hash`. This field corresponds to
    /// `astria.SequencerBlock.rollup_ids_proof`.
    pub rollup_ids_proof: merkle::Proof,
}

impl UncheckedSubmittedMetadata {
    /// Converts this unchecked blob into its checked [`SubmittedMetadata`] representation.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_into_celestia_sequencer_blob(
        self,
    ) -> Result<SubmittedMetadata, SubmittedMetadataError> {
        SubmittedMetadata::try_from_unchecked(self)
    }

    /// Converts from the raw decoded protobuf representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(raw: raw::SubmittedMetadata) -> Result<Self, SubmittedMetadataError> {
        let raw::SubmittedMetadata {
            block_hash,
            header,
            rollup_ids,
            rollup_transactions_proof,
            rollup_ids_proof,
            ..
        } = raw;
        let header = 'header: {
            let Some(header) = header else {
                break 'header Err(SubmittedMetadataError::field_not_set("header"));
            };
            SequencerBlockHeader::try_from_raw(header).map_err(SubmittedMetadataError::header)
        }?;
        let rollup_ids: Vec<_> = rollup_ids
            .iter()
            .map(RollupId::try_from_raw)
            .collect::<Result<_, _>>()
            .map_err(SubmittedMetadataError::rollup_ids)?;

        let rollup_transactions_proof = 'transactions_proof: {
            let Some(rollup_transactions_proof) = rollup_transactions_proof else {
                break 'transactions_proof Err(SubmittedMetadataError::field_not_set(
                    "rollup_transactions_root",
                ));
            };
            merkle::Proof::try_from_raw(rollup_transactions_proof)
                .map_err(SubmittedMetadataError::rollup_transactions_proof)
        }?;

        let rollup_ids_proof = 'ids_proof: {
            let Some(rollup_ids_proof) = rollup_ids_proof else {
                break 'ids_proof Err(SubmittedMetadataError::field_not_set("rollup_ids_proof"));
            };
            merkle::Proof::try_from_raw(rollup_ids_proof)
                .map_err(SubmittedMetadataError::rollup_ids_proof)
        }?;

        let block_hash = block_hash
            .as_ref()
            .try_into()
            .map_err(|_| SubmittedMetadataError::block_hash(block_hash.len()))?;

        Ok(Self {
            block_hash,
            header,
            rollup_ids,
            rollup_transactions_proof,
            rollup_ids_proof,
        })
    }
}

#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct SubmittedMetadata {
    /// The block hash obtained from hashing `.header`.
    block_hash: [u8; 32],
    /// The sequencer block header.
    header: SequencerBlockHeader,
    /// The rollup IDs for which `SubmittedRollupData`s were submitted to celestia.
    /// Corresponds to the `astria.sequencer.v1.RollupTransactions.id` field
    /// and is extracted from `astria.SequencerBlock.rollup_transactions`.
    rollup_ids: Vec<RollupId>,
    /// The proof that the rollup transactions are included in sequencer block.
    /// Corresponds to `astria.SequencerBlock.rollup_transactions_proof`.
    rollup_transactions_proof: merkle::Proof,
    /// The proof that this sequencer blob includes all rollup IDs of the original sequencer
    /// block it was derived from. This proof together with `Sha256(MTH(rollup_ids))` (Sha256
    /// applied to the Merkle Tree Hash of the rollup ID sequence) must be equal to
    /// `header.data_hash` which itself must match
    /// `astria.SequencerBlock.header.data_hash`. This field corresponds to
    /// `astria.SequencerBlock.rollup_ids_proof`.
    rollup_ids_proof: merkle::Proof,
}

/// An iterator over rollup IDs.
pub struct RollupIdIter<'a>(std::slice::Iter<'a, RollupId>);

impl<'a> Iterator for RollupIdIter<'a> {
    type Item = &'a RollupId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl SubmittedMetadata {
    /// Returns the block hash of the tendermint header stored in this blob.
    #[must_use]
    pub fn block_hash(&self) -> &[u8; 32] {
        &self.block_hash
    }

    /// Returns the sequencer's `CometBFT` chain ID.
    #[must_use]
    pub fn cometbft_chain_id(&self) -> &tendermint::chain::Id {
        self.header.chain_id()
    }

    /// Returns the `CometBFT` height stored in the header of the [`SequencerBlock`] this blob was
    /// derived from.
    #[must_use]
    pub fn height(&self) -> tendermint::block::Height {
        self.header.height()
    }

    /// Returns the header of the [`SequencerBlock`] this blob was derived from.
    #[must_use]
    pub fn header(&self) -> &SequencerBlockHeader {
        &self.header
    }

    /// Returns the rollup IDs.
    #[must_use]
    pub fn rollup_ids(&self) -> RollupIdIter {
        RollupIdIter(self.rollup_ids.iter())
    }

    /// Returns the Merkle Tree Hash constructed from the rollup transactions of the original
    /// [`SequencerBlock`] this blob was derived from.
    #[must_use]
    pub fn rollup_transactions_root(&self) -> &[u8; 32] {
        self.header.rollup_transactions_root()
    }

    #[must_use]
    pub fn contains_rollup_id(&self, rollup_id: RollupId) -> bool {
        self.rollup_ids.contains(&rollup_id)
    }

    /// Converts into the unchecked representation fo this type.
    #[must_use]
    pub fn into_unchecked(self) -> UncheckedSubmittedMetadata {
        let Self {
            block_hash,
            header,
            rollup_ids,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = self;
        UncheckedSubmittedMetadata {
            block_hash,
            header,
            rollup_ids,
            rollup_transactions_proof,
            rollup_ids_proof,
        }
    }

    /// Converts from the unchecked representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_unchecked(
        unchecked: UncheckedSubmittedMetadata,
    ) -> Result<Self, SubmittedMetadataError> {
        let UncheckedSubmittedMetadata {
            block_hash,
            header,
            rollup_ids,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = unchecked;

        if !rollup_transactions_proof.verify(
            &Sha256::digest(header.rollup_transactions_root()),
            *header.data_hash(),
        ) {
            return Err(SubmittedMetadataError::rollup_transactions_not_in_cometbft_block());
        }

        if !super::are_rollup_ids_included(
            rollup_ids.iter().copied(),
            &rollup_ids_proof,
            *header.data_hash(),
        ) {
            return Err(SubmittedMetadataError::rollup_ids_not_in_cometbft_block());
        }

        Ok(Self {
            block_hash,
            header,
            rollup_ids,
            rollup_transactions_proof,
            rollup_ids_proof,
        })
    }

    /// Converts into the raw decoded protobuf representation of this type.
    pub fn into_raw(self) -> raw::SubmittedMetadata {
        let Self {
            block_hash,
            header,
            rollup_ids,
            rollup_transactions_proof,
            rollup_ids_proof,
            ..
        } = self;
        raw::SubmittedMetadata {
            block_hash: Bytes::copy_from_slice(&block_hash),
            header: Some(header.into_raw()),
            rollup_ids: rollup_ids.into_iter().map(RollupId::into_raw).collect(),
            rollup_transactions_proof: Some(rollup_transactions_proof.into_raw()),
            rollup_ids_proof: Some(rollup_ids_proof.into_raw()),
        }
    }

    /// Converts from the raw decoded protobuf representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(raw: raw::SubmittedMetadata) -> Result<Self, SubmittedMetadataError> {
        UncheckedSubmittedMetadata::try_from_raw(raw)
            .and_then(UncheckedSubmittedMetadata::try_into_celestia_sequencer_blob)
    }
}
