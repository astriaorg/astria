use sha2::{
    Digest as _,
    Sha256,
};

use super::{
    block::{
        RollupTransactionsParts,
        SequencerBlockHeaderParts,
    },
    raw,
    IncorrectRollupIdLength,
    RollupId,
};
use crate::sequencer::Protobuf;

/// A bundle of blobs constructed from a [`super::SequencerBlock`].
///
/// Consists of a head [`CelestiaSequencerBlob`] and a tail of [`CelestiaRollupBlob`]s.
/// Used as a pass-through data structure to
pub(super) struct CelestiaBlobBundle {
    head: CelestiaSequencerBlob,
    tail: Vec<CelestiaRollupBlob>,
}

impl CelestiaBlobBundle {
    /// Construct a bundle of celestia blobs from a [`super::SequencerBlock`].
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // the proofs are guaranteed to exist; revisit if refactoring
    pub(super) fn from_sequencer_block(block: super::SequencerBlock) -> Self {
        let block_hash = block.block_hash();
        let super::block::SequencerBlockParts {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = block.into_parts();

        let SequencerBlockHeaderParts {
            cometbft_header,
            rollup_transactions_root,
            ..
        } = header.into_parts();

        let head = CelestiaSequencerBlob {
            block_hash,
            header: cometbft_header,
            rollup_ids: rollup_transactions.keys().copied().collect(),
            rollup_transactions_root,
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
            tail.push(CelestiaRollupBlob {
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

    /// Returns the head and the tail of the celestia blob bundle, consuming it.
    pub(super) fn into_parts(self) -> (CelestiaSequencerBlob, Vec<CelestiaRollupBlob>) {
        (self.head, self.tail)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed constructing a celestia rollup blob")]
#[allow(clippy::module_name_repetitions)]
pub struct CelestiaRollupBlobError {
    #[source]
    kind: CelestiaRollupBlobErrorKind,
}

impl CelestiaRollupBlobError {
    fn field_not_set(field: &'static str) -> Self {
        Self {
            kind: CelestiaRollupBlobErrorKind::FieldNotSet {
                field,
            },
        }
    }

    fn rollup_id(source: IncorrectRollupIdLength) -> Self {
        Self {
            kind: CelestiaRollupBlobErrorKind::RollupId {
                source,
            },
        }
    }

    fn proof(source: <merkle::Proof as Protobuf>::Error) -> Self {
        Self {
            kind: CelestiaRollupBlobErrorKind::Proof {
                source,
            },
        }
    }

    fn sequencer_block_hash(actual_len: usize) -> Self {
        Self {
            kind: CelestiaRollupBlobErrorKind::SequencerBlockHash(actual_len),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum CelestiaRollupBlobErrorKind {
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

/// A shadow of [`CelestiaRollupBlob`] with public access to all its fields.
///
/// At the moment there are no invariants upheld by [`CelestiaRollupBlob`] so
/// they can be converted directly into one another. This can change in the future.
pub struct UncheckedCelestiaRollupBlob {
    /// The hash of the sequencer block. Must be 32 bytes.
    pub sequencer_block_hash: [u8; 32],
    /// The 32 bytes identifying the rollup this blob belongs to. Matches
    /// `astria.sequencer.v1alpha1.RollupTransactions.rollup_id`
    pub rollup_id: RollupId,
    /// A list of opaque bytes that are serialized rollup transactions.
    pub transactions: Vec<Vec<u8>>,
    /// The proof that these rollup transactions are included in sequencer block.
    pub proof: merkle::Proof,
}

impl UncheckedCelestiaRollupBlob {
    #[must_use]
    pub fn into_celestia_rollup_blob(self) -> CelestiaRollupBlob {
        CelestiaRollupBlob::from_unchecked(self)
    }
}

#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct CelestiaRollupBlob {
    /// The hash of the sequencer block. Must be 32 bytes.
    sequencer_block_hash: [u8; 32],
    /// The 32 bytes identifying the rollup this blob belongs to. Matches
    /// `astria.sequencer.v1alpha1.RollupTransactions.rollup_id`
    rollup_id: RollupId,
    /// A list of opaque bytes that are serialized rollup transactions.
    transactions: Vec<Vec<u8>>,
    /// The proof that these rollup transactions are included in sequencer block.
    proof: merkle::Proof,
}

impl CelestiaRollupBlob {
    #[must_use]
    pub fn proof(&self) -> &merkle::Proof {
        &self.proof
    }

    #[must_use]
    pub fn transactions(&self) -> &[Vec<u8>] {
        &self.transactions
    }

    #[must_use]
    pub fn rollup_id(&self) -> RollupId {
        self.rollup_id
    }

    #[must_use]
    pub fn sequencer_block_hash(&self) -> [u8; 32] {
        self.sequencer_block_hash
    }

    /// Converts from the unchecked representation of this type (its shadow).
    ///
    /// This type does not uphold any extra invariants so there are no extra checks necessary.
    #[must_use]
    pub fn from_unchecked(unchecked: UncheckedCelestiaRollupBlob) -> Self {
        let UncheckedCelestiaRollupBlob {
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
    pub fn into_unchecked(self) -> UncheckedCelestiaRollupBlob {
        let Self {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        } = self;
        UncheckedCelestiaRollupBlob {
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
    pub fn into_raw(self) -> raw::CelestiaRollupBlob {
        let Self {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        } = self;
        raw::CelestiaRollupBlob {
            sequencer_block_hash: sequencer_block_hash.to_vec(),
            rollup_id: rollup_id.to_vec(),
            transactions,
            proof: Some(proof.into_raw()),
        }
    }

    /// Converts from the raw decoded protobuf representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(raw: raw::CelestiaRollupBlob) -> Result<Self, CelestiaRollupBlobError> {
        let raw::CelestiaRollupBlob {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        } = raw;
        let rollup_id =
            RollupId::try_from_vec(rollup_id).map_err(CelestiaRollupBlobError::rollup_id)?;
        let sequencer_block_hash = sequencer_block_hash
            .try_into()
            .map_err(|bytes: Vec<u8>| CelestiaRollupBlobError::sequencer_block_hash(bytes.len()))?;
        let proof = 'proof: {
            let Some(proof) = proof else {
                break 'proof Err(CelestiaRollupBlobError::field_not_set("proof"));
            };
            merkle::Proof::try_from_raw(proof).map_err(CelestiaRollupBlobError::proof)
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
pub struct CelestiaSequencerBlobError {
    #[source]
    kind: CelestiaSequencerBlobErrorKind,
}

impl CelestiaSequencerBlobError {
    fn empty_cometbft_block_hash() -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::EmptyCometBftBlockHash,
        }
    }

    fn cometbft_header(source: tendermint::Error) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::CometBftHeader {
                source,
            },
        }
    }

    fn field_not_set(field: &'static str) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::FieldNotSet(field),
        }
    }

    fn rollup_ids(source: IncorrectRollupIdLength) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupIds {
                source,
            },
        }
    }

    fn rollup_transactions_root(actual_len: usize) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupTransactionsRoot(actual_len),
        }
    }

    fn rollup_transactions_proof(source: <merkle::Proof as Protobuf>::Error) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupTransactionsProof {
                source,
            },
        }
    }

    fn rollup_ids_proof(source: <merkle::Proof as Protobuf>::Error) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupIdsProof {
                source,
            },
        }
    }

    fn rollup_transactions_not_in_cometbft_block() -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupTransactionsNotInCometBftBlock,
        }
    }

    fn rollup_ids_not_in_cometbft_block() -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupIdsNotInCometBftBlock,
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum CelestiaSequencerBlobErrorKind {
    #[error("the hash derived from the cometbft header was empty where it should be 32 bytes")]
    EmptyCometBftBlockHash,
    #[error("failed constructing the cometbft header from its raw source value")]
    CometBftHeader { source: tendermint::Error },
    #[error("the field of the raw source value was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("one of the rollup IDs in the raw source value was invalid")]
    RollupIds { source: IncorrectRollupIdLength },
    #[error(
        "the provided bytes were too short for a rollup transactions Merkle Tree Hash; expected: \
         32 bytes, actual: {0} bytes"
    )]
    RollupTransactionsRoot(usize),
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

/// A shadow of [`CelestiaSequencerBlob`] with public access to its fields.
///
/// This type does not guarantee any invariants and is mainly useful to get
/// access the sequencer block's internal types.
#[derive(Clone, Debug)]
pub struct UncheckedCelestiaSequencerBlob {
    /// The original `CometBFT` header that is the input to this blob's original sequencer block.
    /// Corresponds to `astria.SequencerBlock.header`.
    pub header: tendermint::block::header::Header,
    /// The rollup rollup IDs for which `CelestiaRollupBlob`s were submitted to celestia.
    /// Corresponds to the `astria.sequencer.v1alpha1.RollupTransactions.id` field
    /// and is extracted from `astria.SequencerBlock.rollup_transactions`.
    pub rollup_ids: Vec<RollupId>,
    /// The Merkle Tree Hash of the rollup transactions. Corresponds to
    /// `MHT(astria.SequencerBlock.rollup_transactions)`, the Merkle
    /// Tree Hash deriveed from the rollup transactions.
    /// Always 32 bytes.
    pub rollup_transactions_root: [u8; 32],
    /// The proof that the rollup transactions are included in sequencer block.
    /// Corresponds to `astria.SequencerBlock.rollup_transactions_proof`.
    pub rollup_transactions_proof: merkle::Proof,
    /// The proof that this sequencer blob includes all rollup IDs of the original sequencer
    /// block it was derived from. This proof together with `Sha256(MHT(rollup_ids))` (Sha256
    /// applied to the Merkle Tree Hash of the rollup ID sequence) must be equal to
    /// `header.data_hash` which itself must match
    /// `astria.SequencerBlock.header.data_hash`. This field corresponds to
    /// `astria.SequencerBlock.rollup_ids_proof`.
    pub rollup_ids_proof: merkle::Proof,
}

impl UncheckedCelestiaSequencerBlob {
    /// Converts this unchecked blob into its checked [`CelestiaSequencerBlob`] representation.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_into_celestia_sequencer_blob(
        self,
    ) -> Result<CelestiaSequencerBlob, CelestiaSequencerBlobError> {
        CelestiaSequencerBlob::try_from_unchecked(self)
    }

    /// Converts from the raw decoded protobuf representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(
        raw: raw::CelestiaSequencerBlob,
    ) -> Result<Self, CelestiaSequencerBlobError> {
        let raw::CelestiaSequencerBlob {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = raw;
        let header = 'cometbft_header: {
            let Some(header) = header else {
                break 'cometbft_header Err(CelestiaSequencerBlobError::field_not_set("header"));
            };
            tendermint::block::Header::try_from(header)
                .map_err(CelestiaSequencerBlobError::cometbft_header)
        }?;
        let rollup_ids: Vec<_> = rollup_ids
            .into_iter()
            .map(RollupId::try_from_vec)
            .collect::<Result<_, _>>()
            .map_err(CelestiaSequencerBlobError::rollup_ids)?;

        let rollup_transactions_root =
            rollup_transactions_root
                .try_into()
                .map_err(|bytes: Vec<_>| {
                    CelestiaSequencerBlobError::rollup_transactions_root(bytes.len())
                })?;

        let rollup_transactions_proof = 'transactions_proof: {
            let Some(rollup_transactions_proof) = rollup_transactions_proof else {
                break 'transactions_proof Err(CelestiaSequencerBlobError::field_not_set(
                    "rollup_transactions_root",
                ));
            };
            merkle::Proof::try_from_raw(rollup_transactions_proof)
                .map_err(CelestiaSequencerBlobError::rollup_transactions_proof)
        }?;

        let rollup_ids_proof = 'ids_proof: {
            let Some(rollup_ids_proof) = rollup_ids_proof else {
                break 'ids_proof Err(CelestiaSequencerBlobError::field_not_set(
                    "rollup_ids_proof",
                ));
            };
            merkle::Proof::try_from_raw(rollup_ids_proof)
                .map_err(CelestiaSequencerBlobError::rollup_ids_proof)
        }?;

        Ok(Self {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        })
    }
}

#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct CelestiaSequencerBlob {
    /// The block hash obtained from hashing `.header`.
    block_hash: [u8; 32],
    /// The original `CometBFT` header that is the input to this blob's original sequencer block.
    /// Corresponds to `astria.SequencerBlock.header`.
    header: tendermint::block::header::Header,
    /// The rollup IDs for which `CelestiaRollupBlob`s were submitted to celestia.
    /// Corresponds to the `astria.sequencer.v1alpha1.RollupTransactions.id` field
    /// and is extracted from `astria.SequencerBlock.rollup_transactions`.
    rollup_ids: Vec<RollupId>,
    /// The Merkle Tree Hash of the rollup transactions. Corresponds to
    /// `MHT(astria.SequencerBlock.rollup_transactions)`, the Merkle
    /// Tree Hash deriveed from the rollup transactions.
    /// Always 32 bytes.
    rollup_transactions_root: [u8; 32],
    /// The proof that the rollup transactions are included in sequencer block.
    /// Corresponds to `astria.SequencerBlock.rollup_transactions_proof`.
    rollup_transactions_proof: merkle::Proof,
    /// The proof that this sequencer blob includes all rollup IDs of the original sequencer
    /// block it was derived from. This proof together with `Sha256(MHT(rollup_ids))` (Sha256
    /// applied to the Merkle Tree Hash of the rollup ID sequence) must be equal to
    /// `header.data_hash` which itself must match
    /// `astria.SequencerBlock.header.data_hash`. This field corresponds to
    /// `astria.SequencerBlock.rollup_ids_proof`.
    rollup_ids_proof: merkle::Proof,
}

impl CelestiaSequencerBlob {
    /// Returns the block hash of the tendermint header stored in this blob.
    #[must_use]
    pub fn block_hash(&self) -> [u8; 32] {
        self.block_hash
    }

    /// Returns the sequencer's `CometBFT` chain ID.
    #[must_use]
    pub fn cometbft_chain_id(&self) -> &tendermint::chain::Id {
        &self.header.chain_id
    }

    /// Returns the `CometBFT` height stored in the header of the [`SequencerBlock`] this blob was
    /// derived from.
    #[must_use]
    pub fn height(&self) -> tendermint::block::Height {
        self.header.height
    }

    /// Returns the `CometBFT` header of the [`SequencerBlock`] this blob was derived from.
    #[must_use]
    pub fn header(&self) -> &tendermint::block::Header {
        &self.header
    }

    #[must_use]
    pub fn contains_rollup_id(&self, rollup_id: RollupId) -> bool {
        self.rollup_ids.contains(&rollup_id)
    }

    /// Returns the Merkle Tree Hash constructed from the rollup transactions of the original
    /// [`SequencerBlock`] this blob was derived from.
    #[must_use]
    pub fn rollup_transactions_root(&self) -> [u8; 32] {
        self.rollup_transactions_root
    }

    /// Converts into the unchecked representation fo this type.
    #[must_use]
    pub fn into_unchecked(self) -> UncheckedCelestiaSequencerBlob {
        let Self {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
            ..
        } = self;
        UncheckedCelestiaSequencerBlob {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        }
    }

    /// Converts from the unchecked representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_unchecked(
        unchecked: UncheckedCelestiaSequencerBlob,
    ) -> Result<Self, CelestiaSequencerBlobError> {
        let UncheckedCelestiaSequencerBlob {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = unchecked;
        let tendermint::Hash::Sha256(block_hash) = header.hash() else {
            return Err(CelestiaSequencerBlobError::empty_cometbft_block_hash());
        };
        // header.data_hash is Option<Hash> and Hash itself has
        // variants Sha256([u8; 32]) or None.
        let Some(tendermint::Hash::Sha256(data_hash)) = header.data_hash else {
            return Err(CelestiaSequencerBlobError::field_not_set(
                "header.data_hash",
            ));
        };

        if !rollup_transactions_proof.verify(&Sha256::digest(rollup_transactions_root), data_hash) {
            return Err(CelestiaSequencerBlobError::rollup_transactions_not_in_cometbft_block());
        }

        if !super::are_rollup_ids_included(rollup_ids.iter().copied(), &rollup_ids_proof, data_hash)
        {
            return Err(CelestiaSequencerBlobError::rollup_ids_not_in_cometbft_block());
        }

        Ok(Self {
            block_hash,
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        })
    }

    /// Converts into the raw decoded protobuf representation of this type.
    pub fn into_raw(self) -> raw::CelestiaSequencerBlob {
        let Self {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
            ..
        } = self;
        raw::CelestiaSequencerBlob {
            header: Some(header.into()),
            rollup_ids: rollup_ids.into_iter().map(RollupId::to_vec).collect(),
            rollup_transactions_root: rollup_transactions_root.to_vec(),
            rollup_transactions_proof: Some(rollup_transactions_proof.into_raw()),
            rollup_ids_proof: Some(rollup_ids_proof.into_raw()),
        }
    }

    /// Converts from the raw decoded protobuf representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(
        raw: raw::CelestiaSequencerBlob,
    ) -> Result<Self, CelestiaSequencerBlobError> {
        UncheckedCelestiaSequencerBlob::try_from_raw(raw)
            .and_then(UncheckedCelestiaSequencerBlob::try_into_celestia_sequencer_blob)
    }
}
