use indexmap::IndexMap;
use sha2::{
    Digest as _,
    Sha256,
};

use super::{
    are_rollup_ids_included,
    derive_merkle_tree_from_rollup_txs,
    Action,
    CelestiaRollupBlob,
    CelestiaSequencerBlob,
    IncorrectRollupIdLength,
    RollupId,
    SequenceAction,
    SignedTransaction,
    SignedTransactionError,
};
use crate::{
    generated::sequencer::v1alpha1 as raw,
    native::Protobuf as _,
};

#[derive(Debug)]
pub enum RollupTransactionsError {
    RollupId(IncorrectRollupIdLength),
}

/// The opaque transactions belonging to a rollup identified by its rollup ID.
#[derive(Clone)]
pub struct RollupTransactions {
    /// The 32 bytes identifying a rollup. Usually the sha256 hash of a plain rollup name.
    id: RollupId,
    /// The serialized opaque bytes of the rollup transactions.
    transactions: Vec<Vec<u8>>,
}

impl RollupTransactions {
    /// Returns the [`RollupId`] identifying the rollup these transactions belong to.
    #[must_use]
    pub fn id(&self) -> RollupId {
        self.id
    }

    /// Returns the opaque transactions bytes.
    #[must_use]
    pub fn transactions(&self) -> &[Vec<u8>] {
        &self.transactions
    }

    /// Transforms these rollup transactions into their raw representation, which can in turn be
    /// encoded as protobuf.
    #[must_use]
    pub fn into_raw(self) -> raw::RollupTransactions {
        let Self {
            id,
            transactions,
        } = self;
        raw::RollupTransactions {
            id: id.get().to_vec(),
            transactions,
        }
    }

    /// Attempts to transform the rollup transactions from their raw representation.
    ///
    /// # Errors
    /// Returns an error if the rollup ID bytes could not be turned into a [`RollupId`].
    pub fn try_from_raw(raw: raw::RollupTransactions) -> Result<Self, RollupTransactionsError> {
        let raw::RollupTransactions {
            id,
            transactions,
        } = raw;
        let id = RollupId::try_from_slice(&id).map_err(RollupTransactionsError::RollupId)?;
        Ok(Self {
            id,
            transactions,
        })
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, thiserror::Error)]
pub enum SequencerBlockError {
    #[error(
        "the CometBFT block.header.data_hash does not match the Merkle Tree Hash derived from \
         block.data"
    )]
    CometBftDataHashDoesNotMatchReconstructed,
    #[error("hashing the CometBFT block.header returned an empty hash which is not permitted")]
    CometBftBlockHashIsNone,
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("failed creating a native cometbft Header from the raw protobuf header")]
    Header(#[source] tendermint::Error),
    #[error(
        "failed parsing a raw protobuf rollup transaction because it contained an invalid rollup \
         ID"
    )]
    ParseRollupTransactions(#[source] IncorrectRollupIdLength),
    #[error("failed constructing a transaction proof from the raw protobuf transaction proof")]
    TransactionProofInvalid(#[source] merkle::audit::InvalidProof),
    #[error("failed constructing a rollup ID proof from the raw protobuf rollup ID proof")]
    IdProofInvalid(#[source] merkle::audit::InvalidProof),
    #[error(
        "the cometbft block.data field was too short and did not contain the rollup transaction \
         root"
    )]
    NoRollupTransactionsRoot,
    #[error(
        "the rollup transaction root in the cometbft block.data field was expected to be 32 bytes \
         long, but was actually `{0}`"
    )]
    IncorrectRollupTransactionsRootLength(usize),
    #[error("the cometbft block.data field was too short and did not contain the rollup ID root")]
    NoRollupIdsRoot,
    #[error(
        "the rollup ID root in the cometbft block.data field was expected to be 32 bytes long, \
         but was actually `{0}`"
    )]
    IncorrectRollupIdsRootLength(usize),
    #[error(
        "the Merkle Tree Hash derived from the rollup transactions recorded in the raw protobuf \
         sequencer block could not be verified against their proof and the block's data hash"
    )]
    RollupTransactionsNotInSequencerBlock,
    #[error(
        "the Merkle Tree Hash derived from the rollup IDs recorded in the raw protobuf sequencer \
         block could not be verified against their proof and the block's data hash"
    )]
    RollupIdsNotInSequencerBlock,
    #[error(
        "failed decoding an entry in the cometbft block.data field as a protobuf signed astria \
         transaction"
    )]
    SignedTransactionProtobufDecode(#[source] prost::DecodeError),
    #[error(
        "failed converting a raw protobuf signed transaction decoded from the cometbft block.data
        field to a native astria signed transaction"
    )]
    RawSignedTransactionConversion(#[source] SignedTransactionError),
    #[error(
        "the root derived from the rollup transactions in the cometbft block.data field did not \
         match the root stored in the same block.data field"
    )]
    RollupTransactionsRootDoesNotMatchReconstructed,
    #[error(
        "the root derived from the rollup IDs in the cometbft block.data field did not match the \
         root stored in the same block.data field"
    )]
    RollupIdsRootDoesNotMatchReconstructed,
}

/// A shadow of [`SequencerBlock`] with full public access to its fields.
///
/// This type does not guarantee any invariants and is mainly useful to get
/// access the sequencer block's internal types.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
pub struct UncheckedSequencerBlock {
    /// The original `CometBFT` header that was the input to this sequencer block.
    pub header: tendermint::block::header::Header,
    /// The collection of rollup transactions that were included in this block.
    pub rollup_transactions: IndexMap<RollupId, Vec<Vec<u8>>>,
    // The proof that the rollup transactions are included in the `CometBFT` block this
    // sequencer block is derived form. This proof together with
    // `Sha256(MTH(rollup_transactions))` must match `header.data_hash`.
    // `MTH(rollup_transactions)` is the Merkle Tree Hash derived from the
    // rollup transactions.
    pub rollup_transactions_proof: merkle::Proof,
    // The proof that the rollup IDs listed in `rollup_transactions` are included
    // in the `CometBFT` block this sequencer block is derived form. This proof together
    // with `Sha256(MTH(rollup_ids))` must match `header.data_hash`.
    // `MTH(rollup_ids)` is the Merkle Tree Hash derived from the rollup IDs listed in
    // the rollup transactions.
    pub rollup_ids_proof: merkle::Proof,
}

/// `SequencerBlock` is constructed from a tendermint/cometbft block by
/// converting its opaque `data` bytes into sequencer specific types.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, PartialEq)]
pub struct SequencerBlock {
    /// The result of hashing `header`. Guaranteed to not be `None` as compared to
    /// the cometbft/tendermint-rs return type.
    block_hash: [u8; 32],
    /// The original `CometBFT` header that was the input to this sequencer block.
    header: tendermint::block::header::Header,
    /// The collection of rollup transactions that were included in this block.
    rollup_transactions: IndexMap<RollupId, Vec<Vec<u8>>>,
    // The proof that the rollup transactions are included in the `CometBFT` block this
    // sequencer block is derived form. This proof together with
    // `Sha256(MTH(rollup_transactions))` must match `header.data_hash`.
    // `MTH(rollup_transactions)` is the Merkle Tree Hash derived from the
    // rollup transactions.
    rollup_transactions_proof: merkle::Proof,
    // The proof that the rollup IDs listed in `rollup_transactions` are included
    // in the `CometBFT` block this sequencer block is derived form. This proof together
    // with `Sha256(MTH(rollup_ids))` must match `header.data_hash`.
    // `MTH(rollup_ids)` is the Merkle Tree Hash derived from the rollup IDs listed in
    // the rollup transactions.
    rollup_ids_proof: merkle::Proof,
}

impl SequencerBlock {
    /// Returns the hash of the `CometBFT` block this sequencer block is derived from.
    ///
    /// This is done by hashing the `CometBFT` header stored in this block.
    #[must_use]
    pub fn block_hash(&self) -> [u8; 32] {
        self.block_hash
    }

    #[must_use]
    pub fn header(&self) -> &tendermint::block::header::Header {
        &self.header
    }

    #[must_use]
    pub fn rollup_transactions(&self) -> &IndexMap<RollupId, Vec<Vec<u8>>> {
        &self.rollup_transactions
    }

    #[must_use]
    pub fn into_raw(self) -> raw::SequencerBlock {
        fn tuple_to_rollup_txs(
            (rollup_id, transactions): (RollupId, Vec<Vec<u8>>),
        ) -> raw::RollupTransactions {
            raw::RollupTransactions {
                id: rollup_id.to_vec(),
                transactions,
            }
        }

        let Self {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
            ..
        } = self;
        raw::SequencerBlock {
            header: Some(header.into()),
            rollup_transactions: rollup_transactions
                .into_iter()
                .map(tuple_to_rollup_txs)
                .collect(),
            rollup_transactions_proof: Some(rollup_transactions_proof.into_raw()),
            rollup_ids_proof: Some(rollup_ids_proof.into_raw()),
        }
    }

    #[must_use]
    pub fn into_unchecked(self) -> UncheckedSequencerBlock {
        let Self {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
            ..
        } = self;
        UncheckedSequencerBlock {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        }
    }

    /// Turn the sequencer block into a [`CelestiaSequencerBlob`] and its associated list of
    /// [`CelestiaRollupBlob`]s.
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // the proofs are guaranteed to exist; revisit if refactoring
    pub fn to_celestia_blobs(&self) -> (CelestiaSequencerBlob, Vec<CelestiaRollupBlob>) {
        let SequencerBlock {
            block_hash,
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = self;

        let tree = derive_merkle_tree_from_rollup_txs(rollup_transactions);

        let head = CelestiaSequencerBlob {
            block_hash: *block_hash,
            header: header.clone(),
            rollup_ids: rollup_transactions.keys().copied().collect(),
            rollup_transactions_root: tree.root(),
            rollup_transactions_proof: rollup_transactions_proof.clone(),
            rollup_ids_proof: rollup_ids_proof.clone(),
        };

        let mut tail = Vec::with_capacity(self.rollup_transactions.len());
        for (i, (rollup_id, transactions)) in self.rollup_transactions.iter().enumerate() {
            let proof = tree
                .construct_proof(i)
                .expect("the proof must exist because the tree was derived with the same leaf");
            tail.push(CelestiaRollupBlob {
                sequencer_block_hash: self.block_hash(),
                rollup_id: *rollup_id,
                transactions: transactions.clone(),
                proof,
            });
        }
        (head, tail)
    }

    /// Converts from a [`tendermint::Block`].
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    #[allow(clippy::missing_panics_doc)] // the panic sources are checked before hand; revisit if refactoring
    pub fn try_from_cometbft(block: tendermint::Block) -> Result<Self, SequencerBlockError> {
        use prost::Message as _;

        let tendermint::Block {
            header,
            data,
            ..
        } = block;

        let Some(tendermint::Hash::Sha256(data_hash)) = header.data_hash else {
            // header.data_hash is Option<Hash> and Hash itself has
            // variants Sha256([u8; 32]) or None.
            return Err(SequencerBlockError::FieldNotSet("header.data_hash"));
        };

        let tendermint::Hash::Sha256(block_hash) = header.hash() else {
            return Err(SequencerBlockError::CometBftBlockHashIsNone);
        };

        let tree = merkle_tree_from_data(&data);
        if tree.root() != data_hash {
            return Err(SequencerBlockError::CometBftDataHashDoesNotMatchReconstructed);
        }

        let mut data_list = data.into_iter();
        let rollup_transactions_root: [u8; 32] = data_list
            .next()
            .ok_or(SequencerBlockError::NoRollupTransactionsRoot)?
            .try_into()
            .map_err(|e: Vec<_>| {
                SequencerBlockError::IncorrectRollupTransactionsRootLength(e.len())
            })?;

        let rollup_ids_root: [u8; 32] = data_list
            .next()
            .ok_or(SequencerBlockError::NoRollupIdsRoot)?
            .try_into()
            .map_err(|e: Vec<_>| SequencerBlockError::IncorrectRollupIdsRootLength(e.len()))?;

        let mut rollup_transactions = IndexMap::new();
        for elem in data_list {
            let raw_tx = raw::SignedTransaction::decode(&*elem)
                .map_err(SequencerBlockError::SignedTransactionProtobufDecode)?;
            let signed_tx = SignedTransaction::try_from_raw(raw_tx)
                .map_err(SequencerBlockError::RawSignedTransactionConversion)?;
            for action in signed_tx.transaction.actions {
                if let Action::Sequence(SequenceAction {
                    rollup_id,
                    data,
                }) = action
                {
                    let elem = rollup_transactions.entry(rollup_id).or_insert(vec![]);
                    elem.push(data);
                }
            }
        }
        rollup_transactions.sort_unstable_keys();

        if rollup_transactions_root
            != derive_merkle_tree_from_rollup_txs(&rollup_transactions).root()
        {
            return Err(SequencerBlockError::RollupTransactionsRootDoesNotMatchReconstructed);
        }

        // ensure the rollup IDs commitment matches the one calculated from the rollup data
        if rollup_ids_root != merkle::Tree::from_leaves(rollup_transactions.keys()).root() {
            return Err(SequencerBlockError::RollupIdsRootDoesNotMatchReconstructed);
        }

        // action tree root is always the first tx in a block
        let rollup_transactions_proof = tree.construct_proof(0).expect(
            "the tree has at least one leaf; if this line is reached and `construct_proof` \
             returns None it means that the short circuiting checks above it have been removed",
        );

        let rollup_ids_proof = tree.construct_proof(1).expect(
            "the tree has at least two leaves; if this line is reached and `construct_proof` \
             returns None it means that the short circuiting checks above it have been removed",
        );

        Ok(Self {
            block_hash,
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        })
    }

    /// Converts from the raw decoded protobuf representatin of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(raw: raw::SequencerBlock) -> Result<Self, SequencerBlockError> {
        fn rollup_txs_to_tuple(
            raw: raw::RollupTransactions,
        ) -> Result<(RollupId, Vec<Vec<u8>>), IncorrectRollupIdLength> {
            let id = RollupId::try_from_slice(&raw.id)?;
            Ok((id, raw.transactions))
        }

        let raw::SequencerBlock {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = raw;

        let rollup_transactions_proof = 'proof: {
            let Some(rollup_transactions_proof) = rollup_transactions_proof else {
                break 'proof Err(SequencerBlockError::FieldNotSet(
                    "rollup_transactions_proof",
                ));
            };
            merkle::Proof::try_from_raw(rollup_transactions_proof)
                .map_err(SequencerBlockError::TransactionProofInvalid)
        }?;
        let rollup_ids_proof = 'proof: {
            let Some(rollup_ids_proof) = rollup_ids_proof else {
                break 'proof Err(SequencerBlockError::FieldNotSet("rollup_ids_proof"));
            };
            merkle::Proof::try_from_raw(rollup_ids_proof)
                .map_err(SequencerBlockError::IdProofInvalid)
        }?;
        let header = 'header: {
            let Some(header) = header else {
                break 'header Err(SequencerBlockError::FieldNotSet("header"));
            };
            tendermint::block::Header::try_from(header).map_err(SequencerBlockError::Header)
        }?;
        let tendermint::Hash::Sha256(block_hash) = header.hash() else {
            return Err(SequencerBlockError::CometBftBlockHashIsNone);
        };

        // header.data_hash is Option<Hash> and Hash itself has
        // variants Sha256([u8; 32]) or None.
        let Some(tendermint::Hash::Sha256(data_hash)) = header.data_hash else {
            return Err(SequencerBlockError::FieldNotSet("header.data_hash"));
        };

        let rollup_transactions = rollup_transactions
            .into_iter()
            .map(rollup_txs_to_tuple)
            .collect::<Result<_, _>>()
            .map_err(SequencerBlockError::ParseRollupTransactions)?;

        if !are_rollup_txs_included(&rollup_transactions, &rollup_transactions_proof, data_hash) {
            return Err(SequencerBlockError::RollupTransactionsNotInSequencerBlock);
        }
        if !are_rollup_ids_included(
            rollup_transactions.keys().copied(),
            &rollup_ids_proof,
            data_hash,
        ) {
            return Err(SequencerBlockError::RollupIdsNotInSequencerBlock);
        }

        Ok(Self {
            block_hash,
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        })
    }
}

fn are_rollup_txs_included(
    rollup_txs: &IndexMap<RollupId, Vec<Vec<u8>>>,
    rollup_proof: &merkle::Proof,
    data_hash: [u8; 32],
) -> bool {
    let rollup_tree = derive_merkle_tree_from_rollup_txs(rollup_txs);
    let hash_of_rollup_root = Sha256::digest(rollup_tree.root());
    rollup_proof.verify(&hash_of_rollup_root, data_hash)
}

/// Constructs a `[merkle::Tree]` from an iterator yielding byte slices.
///
/// This hashes each item before pushing it into the Merkle Tree, which
/// effectively causes a double hashing. The leaf hash of an item `d_i`
/// is then `MTH(d_i) = SHA256(0x00 || SHA256(d_i))`.
fn merkle_tree_from_data<I, B>(iter: I) -> merkle::Tree
where
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
{
    merkle::Tree::from_leaves(iter.into_iter().map(|item| Sha256::digest(&item)))
}
