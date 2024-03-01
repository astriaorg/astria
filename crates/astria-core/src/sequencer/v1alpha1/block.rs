use indexmap::IndexMap;
use sha2::Sha256;
use transaction::SignedTransaction;

use super::{
    celestia,
    raw,
    transaction,
    CelestiaRollupBlob,
    CelestiaSequencerBlob,
    IncorrectRollupIdLength,
    RollupId,
};
use crate::{
    sequencer::v1alpha1::{
        are_rollup_ids_included,
        are_rollup_txs_included,
        derive_merkle_tree_from_rollup_txs,
        transaction::action,
    },
    Protobuf as _,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct RollupTransactionsError(RollupTransactionsErrorKind);

impl RollupTransactionsError {
    fn rollup_id(source: IncorrectRollupIdLength) -> Self {
        Self(RollupTransactionsErrorKind::RollupId(source))
    }

    fn field_not_set(field: &'static str) -> Self {
        Self(RollupTransactionsErrorKind::FieldNotSet(field))
    }

    fn proof_invalid(source: merkle::audit::InvalidProof) -> Self {
        Self(RollupTransactionsErrorKind::ProofInvalid(source))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RollupTransactionsErrorKind {
    #[error("`id` field is invalid")]
    RollupId(#[source] IncorrectRollupIdLength),
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("failed constructing a proof from the raw protobuf `proof` field")]
    ProofInvalid(#[source] merkle::audit::InvalidProof),
}

/// The opaque transactions belonging to a rollup identified by its rollup ID.
#[derive(Clone, Debug, PartialEq)]
pub struct RollupTransactions {
    /// The 32 bytes identifying a rollup. Usually the sha256 hash of a plain rollup name.
    id: RollupId,
    /// The serialized opaque bytes of the rollup transactions.
    transactions: Vec<Vec<u8>>,
    /// Proof that this set of transactions belongs in the `sequence::Action` merkle tree
    proof: merkle::Proof,
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

    /// Returns the merkle proof that these transactions were included
    /// in the `action_tree_commitment`.
    #[must_use]
    pub fn proof(&self) -> &merkle::Proof {
        &self.proof
    }

    /// Transforms these rollup transactions into their raw representation, which can in turn be
    /// encoded as protobuf.
    #[must_use]
    pub fn into_raw(self) -> raw::RollupTransactions {
        let Self {
            id,
            transactions,
            proof,
        } = self;
        raw::RollupTransactions {
            id: id.get().to_vec(),
            transactions,
            proof: Some(proof.into_raw()),
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
            proof,
        } = raw;
        let id = RollupId::try_from_slice(&id).map_err(RollupTransactionsError::rollup_id)?;
        let proof = 'proof: {
            let Some(proof) = proof else {
                break 'proof Err(RollupTransactionsError::field_not_set("proof"));
            };
            merkle::Proof::try_from_raw(proof).map_err(RollupTransactionsError::proof_invalid)
        }?;
        Ok(Self {
            id,
            transactions,
            proof,
        })
    }

    #[must_use]
    pub fn into_values(self) -> (RollupId, Vec<Vec<u8>>, merkle::Proof) {
        (self.id, self.transactions, self.proof)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SequencerBlockError(SequencerBlockErrorKind);

impl SequencerBlockError {
    fn comet_bft_data_hash_does_not_match_reconstructed() -> Self {
        Self(SequencerBlockErrorKind::CometBftDataHashDoesNotMatchReconstructed)
    }

    fn comet_bft_block_hash_is_none() -> Self {
        Self(SequencerBlockErrorKind::CometBftBlockHashIsNone)
    }

    fn field_not_set(field: &'static str) -> Self {
        Self(SequencerBlockErrorKind::FieldNotSet(field))
    }

    fn cometbft_header(source: tendermint::Error) -> Self {
        Self(SequencerBlockErrorKind::CometBftHeader(source))
    }

    fn parse_rollup_transactions(source: RollupTransactionsError) -> Self {
        Self(SequencerBlockErrorKind::ParseRollupTransactions(source))
    }

    fn transaction_proof_invalid(source: merkle::audit::InvalidProof) -> Self {
        Self(SequencerBlockErrorKind::TransactionProofInvalid(source))
    }

    fn id_proof_invalid(source: merkle::audit::InvalidProof) -> Self {
        Self(SequencerBlockErrorKind::IdProofInvalid(source))
    }

    fn no_rollup_transactions_root() -> Self {
        Self(SequencerBlockErrorKind::NoRollupTransactionsRoot)
    }

    fn incorrect_rollup_transactions_root_length(len: usize) -> Self {
        Self(SequencerBlockErrorKind::IncorrectRollupTransactionsRootLength(len))
    }

    fn no_rollup_ids_root() -> Self {
        Self(SequencerBlockErrorKind::NoRollupIdsRoot)
    }

    fn incorrect_rollup_ids_root_length(len: usize) -> Self {
        Self(SequencerBlockErrorKind::IncorrectRollupIdsRootLength(len))
    }

    fn rollup_transactions_not_in_sequencer_block() -> Self {
        Self(SequencerBlockErrorKind::RollupTransactionsNotInSequencerBlock)
    }

    fn rollup_ids_not_in_sequencer_block() -> Self {
        Self(SequencerBlockErrorKind::RollupIdsNotInSequencerBlock)
    }

    fn signed_transaction_prootbof_decode(source: prost::DecodeError) -> Self {
        Self(SequencerBlockErrorKind::SignedTransactionProtobufDecode(
            source,
        ))
    }

    fn raw_signed_transaction_conversion(source: transaction::SignedTransactionError) -> Self {
        Self(SequencerBlockErrorKind::RawSignedTransactionConversion(
            source,
        ))
    }

    fn rollup_transactions_root_does_not_match_reconstructed() -> Self {
        Self(SequencerBlockErrorKind::RollupTransactionsRootDoesNotMatchReconstructed)
    }

    fn rollup_ids_root_does_not_match_reconstructed() -> Self {
        Self(SequencerBlockErrorKind::RollupIdsRootDoesNotMatchReconstructed)
    }
}

#[derive(Debug, thiserror::Error)]
enum SequencerBlockErrorKind {
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
    CometBftHeader(#[source] tendermint::Error),
    #[error(
        "failed parsing a raw protobuf rollup transaction because it contained an invalid rollup \
         ID"
    )]
    ParseRollupTransactions(#[source] RollupTransactionsError),
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
    RawSignedTransactionConversion(#[source] transaction::SignedTransactionError),
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
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::module_name_repetitions)]
pub struct UncheckedSequencerBlock {
    /// The original `CometBFT` header that was the input to this sequencer block.
    pub header: tendermint::block::header::Header,
    /// The collection of rollup transactions that were included in this block.
    pub rollup_transactions: IndexMap<RollupId, RollupTransactions>,
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
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::module_name_repetitions)]
pub struct SequencerBlock {
    /// The result of hashing `header`. Guaranteed to not be `None` as compared to
    /// the cometbft/tendermint-rs return type.
    block_hash: [u8; 32],
    /// The original `CometBFT` header that was the input to this sequencer block.
    header: tendermint::block::header::Header,
    /// The collection of rollup transactions that were included in this block.
    rollup_transactions: IndexMap<RollupId, RollupTransactions>,
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

    /// The height stored in this sequencer block.
    #[must_use]
    pub fn height(&self) -> tendermint::block::Height {
        self.header.height
    }

    #[must_use]
    pub fn rollup_transactions(&self) -> &IndexMap<RollupId, RollupTransactions> {
        &self.rollup_transactions
    }

    /// Returns the map of rollup transactions, consuming `self`.
    #[must_use]
    pub fn into_rollup_transactions(self) -> IndexMap<RollupId, RollupTransactions> {
        self.rollup_transactions
    }

    #[must_use]
    pub fn into_raw(self) -> raw::SequencerBlock {
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
                .into_values()
                .map(|tx| tx.into_raw())
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

    #[must_use]
    pub fn into_filtered_block(mut self, rollup_ids: Vec<RollupId>) -> FilteredSequencerBlock {
        let all_rollup_ids: Vec<RollupId> = self.rollup_transactions.keys().copied().collect();

        // recreate the whole rollup tx tree so that we can get the root, as it's not stored
        // in the sequencer block.
        // note: we can remove this after storing the constructed root/proofs in the sequencer app.
        let rollup_transaction_tree = derive_merkle_tree_from_rollup_txs(
            self.rollup_transactions
                .iter()
                .map(|(id, txs)| (id, txs.transactions()))
                .collect::<Vec<(&RollupId, &[Vec<u8>])>>(),
        );

        let mut filtered_rollup_transactions = IndexMap::with_capacity(rollup_ids.len());
        for id in rollup_ids {
            let Some(rollup_transactions) = self.rollup_transactions.shift_remove(&id) else {
                continue;
            };
            filtered_rollup_transactions.insert(id, rollup_transactions);
        }

        FilteredSequencerBlock {
            block_hash: self.block_hash,
            header: self.header,
            rollup_transactions: filtered_rollup_transactions,
            rollup_transactions_root: rollup_transaction_tree.root(),
            rollup_transactions_proof: self.rollup_transactions_proof,
            all_rollup_ids,
            rollup_ids_proof: self.rollup_ids_proof,
        }
    }

    /// Turn the sequencer block into a [`CelestiaSequencerBlob`] and its associated list of
    /// [`CelestiaRollupBlob`]s.
    #[must_use]
    pub fn into_celestia_blobs(self) -> (CelestiaSequencerBlob, Vec<CelestiaRollupBlob>) {
        celestia::CelestiaBlobBundle::from_sequencer_block(self).into_parts()
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
            return Err(SequencerBlockError::field_not_set("header.data_hash"));
        };

        let tendermint::Hash::Sha256(block_hash) = header.hash() else {
            return Err(SequencerBlockError::comet_bft_block_hash_is_none());
        };

        let tree = merkle_tree_from_data(&data);
        if tree.root() != data_hash {
            return Err(SequencerBlockError::comet_bft_data_hash_does_not_match_reconstructed());
        }

        let mut data_list = data.into_iter();
        let rollup_transactions_root: [u8; 32] = data_list
            .next()
            .ok_or(SequencerBlockError::no_rollup_transactions_root())?
            .try_into()
            .map_err(|e: Vec<_>| {
                SequencerBlockError::incorrect_rollup_transactions_root_length(e.len())
            })?;

        let rollup_ids_root: [u8; 32] = data_list
            .next()
            .ok_or(SequencerBlockError::no_rollup_ids_root())?
            .try_into()
            .map_err(|e: Vec<_>| SequencerBlockError::incorrect_rollup_ids_root_length(e.len()))?;

        let mut rollup_base_transactions = IndexMap::new();
        for elem in data_list {
            let raw_tx = raw::SignedTransaction::decode(&*elem)
                .map_err(SequencerBlockError::signed_transaction_prootbof_decode)?;
            let signed_tx = SignedTransaction::try_from_raw(raw_tx)
                .map_err(SequencerBlockError::raw_signed_transaction_conversion)?;
            for action in signed_tx.into_unsigned().actions {
                if let action::Action::Sequence(action::SequenceAction {
                    rollup_id,
                    data,
                    fee_asset_id: _,
                }) = action
                {
                    let elem = rollup_base_transactions.entry(rollup_id).or_insert(vec![]);
                    elem.push(data);
                }
            }
        }
        rollup_base_transactions.sort_unstable_keys();

        // ensure the rollup IDs commitment matches the one calculated from the rollup data
        if rollup_ids_root != merkle::Tree::from_leaves(rollup_base_transactions.keys()).root() {
            return Err(SequencerBlockError::rollup_ids_root_does_not_match_reconstructed());
        }

        let rollup_transaction_tree = derive_merkle_tree_from_rollup_txs(&rollup_base_transactions);
        if rollup_transactions_root != rollup_transaction_tree.root() {
            return Err(
                SequencerBlockError::rollup_transactions_root_does_not_match_reconstructed(),
            );
        }

        let mut rollup_transactions = IndexMap::new();
        for (i, (id, transactions)) in rollup_base_transactions.into_iter().enumerate() {
            let proof = rollup_transaction_tree
                .construct_proof(i)
                .expect("the proof must exist because the tree was derived with the same leaf");
            rollup_transactions.insert(
                id,
                RollupTransactions {
                    id,
                    transactions,
                    proof,
                },
            );
        }
        rollup_transactions.sort_unstable_keys();

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

    /// Converts from the raw decoded protobuf representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(raw: raw::SequencerBlock) -> Result<Self, SequencerBlockError> {
        fn rollup_txs_to_tuple(
            raw: raw::RollupTransactions,
        ) -> Result<(RollupId, RollupTransactions), RollupTransactionsError> {
            let rollup_transactions = RollupTransactions::try_from_raw(raw)?;
            Ok((rollup_transactions.id, rollup_transactions))
        }

        let raw::SequencerBlock {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = raw;

        let rollup_transactions_proof = 'proof: {
            let Some(rollup_transactions_proof) = rollup_transactions_proof else {
                break 'proof Err(SequencerBlockError::field_not_set(
                    "rollup_transactions_proof",
                ));
            };
            merkle::Proof::try_from_raw(rollup_transactions_proof)
                .map_err(SequencerBlockError::transaction_proof_invalid)
        }?;
        let rollup_ids_proof = 'proof: {
            let Some(rollup_ids_proof) = rollup_ids_proof else {
                break 'proof Err(SequencerBlockError::field_not_set("rollup_ids_proof"));
            };
            merkle::Proof::try_from_raw(rollup_ids_proof)
                .map_err(SequencerBlockError::id_proof_invalid)
        }?;
        let header = 'header: {
            let Some(header) = header else {
                break 'header Err(SequencerBlockError::field_not_set("header"));
            };
            tendermint::block::Header::try_from(header)
                .map_err(SequencerBlockError::cometbft_header)
        }?;
        let tendermint::Hash::Sha256(block_hash) = header.hash() else {
            return Err(SequencerBlockError::comet_bft_block_hash_is_none());
        };

        // header.data_hash is Option<Hash> and Hash itself has
        // variants Sha256([u8; 32]) or None.
        let Some(tendermint::Hash::Sha256(data_hash)) = header.data_hash else {
            return Err(SequencerBlockError::field_not_set("header.data_hash"));
        };

        let rollup_transactions = rollup_transactions
            .into_iter()
            .map(rollup_txs_to_tuple)
            .collect::<Result<_, _>>()
            .map_err(SequencerBlockError::parse_rollup_transactions)?;

        if !are_rollup_txs_included(&rollup_transactions, &rollup_transactions_proof, data_hash) {
            return Err(SequencerBlockError::rollup_transactions_not_in_sequencer_block());
        }
        if !are_rollup_ids_included(
            rollup_transactions.keys().copied(),
            &rollup_ids_proof,
            data_hash,
        ) {
            return Err(SequencerBlockError::rollup_ids_not_in_sequencer_block());
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
    use sha2::Digest as _;
    merkle::Tree::from_leaves(iter.into_iter().map(|item| Sha256::digest(&item)))
}

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::module_name_repetitions)]
pub struct FilteredSequencerBlock {
    block_hash: [u8; 32],
    header: tendermint::block::header::Header,
    // filtered set of rollup transactions
    rollup_transactions: IndexMap<RollupId, RollupTransactions>,
    // root of the rollup transactions tree
    rollup_transactions_root: [u8; 32],
    // proof that `rollup_transactions_root` is included in `data_hash`
    rollup_transactions_proof: merkle::Proof,
    // all rollup ids in the sequencer block
    all_rollup_ids: Vec<RollupId>,
    // proof that `rollup_ids` is included in `data_hash`
    rollup_ids_proof: merkle::Proof,
}

impl FilteredSequencerBlock {
    #[must_use]
    pub fn block_hash(&self) -> [u8; 32] {
        self.block_hash
    }

    #[must_use]
    pub fn header(&self) -> &tendermint::block::header::Header {
        &self.header
    }

    #[must_use]
    pub fn height(&self) -> tendermint::block::Height {
        self.header.height
    }

    #[must_use]
    pub fn rollup_transactions(&self) -> &IndexMap<RollupId, RollupTransactions> {
        &self.rollup_transactions
    }

    #[must_use]
    pub fn rollup_transactions_root(&self) -> [u8; 32] {
        self.rollup_transactions_root
    }

    #[must_use]
    pub fn rollup_transactions_proof(&self) -> &merkle::Proof {
        &self.rollup_transactions_proof
    }

    #[must_use]
    pub fn all_rollup_ids(&self) -> &[RollupId] {
        &self.all_rollup_ids
    }

    #[must_use]
    pub fn rollup_ids_proof(&self) -> &merkle::Proof {
        &self.rollup_ids_proof
    }

    #[must_use]
    pub fn into_raw(self) -> raw::FilteredSequencerBlock {
        let Self {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
            ..
        } = self;
        raw::FilteredSequencerBlock {
            header: Some(header.into()),
            rollup_transactions: rollup_transactions
                .into_values()
                .map(|tx| tx.into_raw())
                .collect(),
            rollup_transactions_root: self.rollup_transactions_root.to_vec(),
            rollup_transactions_proof: Some(rollup_transactions_proof.into_raw()),
            all_rollup_ids: self.all_rollup_ids.iter().map(|id| id.to_vec()).collect(),
            rollup_ids_proof: Some(rollup_ids_proof.into_raw()),
        }
    }

    /// Converts from the raw decoded protobuf representation of this type.
    ///
    /// # Errors
    ///
    /// - if the rollup transactions proof is not set
    /// - if the rollup IDs proof is not set
    /// - if the rollup transactions proof cannot be constructed from the raw protobuf
    /// - if the rollup IDs proof cannot be constructed from the raw protobuf
    /// - if the cometbft header is not set
    /// - if the cometbft header cannot be constructed from the raw protobuf
    /// - if the cometbft block hash is None
    /// - if the data hash is None
    /// - if the rollup transactions cannot be parsed
    /// - if the rollup transactions root is not 32 bytes
    /// - if the rollup transactions are not included in the sequencer block
    /// - if the rollup IDs root is not 32 bytes
    /// - if the rollup IDs are not included in the sequencer block
    pub fn try_from_raw(
        raw: raw::FilteredSequencerBlock,
    ) -> Result<Self, FilteredSequencerBlockError> {
        use sha2::Digest as _;

        fn rollup_txs_to_tuple(
            raw: raw::RollupTransactions,
        ) -> Result<(RollupId, RollupTransactions), RollupTransactionsError> {
            let rollup_transactions = RollupTransactions::try_from_raw(raw)?;
            Ok((rollup_transactions.id, rollup_transactions))
        }

        let raw::FilteredSequencerBlock {
            header,
            rollup_transactions,
            rollup_transactions_root,
            rollup_transactions_proof,
            all_rollup_ids,
            rollup_ids_proof,
        } = raw;

        let rollup_transactions_proof = {
            let Some(rollup_transactions_proof) = rollup_transactions_proof else {
                return Err(FilteredSequencerBlockError::field_not_set(
                    "rollup_transactions_proof",
                ));
            };
            merkle::Proof::try_from_raw(rollup_transactions_proof)
                .map_err(FilteredSequencerBlockError::transaction_proof_invalid)
        }?;
        let rollup_ids_proof = {
            let Some(rollup_ids_proof) = rollup_ids_proof else {
                return Err(FilteredSequencerBlockError::field_not_set(
                    "rollup_ids_proof",
                ));
            };
            merkle::Proof::try_from_raw(rollup_ids_proof)
                .map_err(FilteredSequencerBlockError::id_proof_invalid)
        }?;
        let header = {
            let Some(header) = header else {
                return Err(FilteredSequencerBlockError::field_not_set("header"));
            };
            tendermint::block::Header::try_from(header)
                .map_err(FilteredSequencerBlockError::cometbft_header)
        }?;
        let tendermint::Hash::Sha256(block_hash) = header.hash() else {
            return Err(FilteredSequencerBlockError::comet_bft_block_hash_is_none());
        };

        // header.data_hash is Option<Hash> and Hash itself has
        // variants Sha256([u8; 32]) or None.
        let Some(tendermint::Hash::Sha256(data_hash)) = header.data_hash else {
            return Err(FilteredSequencerBlockError::field_not_set(
                "header.data_hash",
            ));
        };

        let rollup_transactions = rollup_transactions
            .into_iter()
            .map(rollup_txs_to_tuple)
            .collect::<Result<_, _>>()
            .map_err(FilteredSequencerBlockError::parse_rollup_transactions)?;

        let rollup_transactions_root: [u8; 32] =
            rollup_transactions_root.try_into().map_err(|e: Vec<_>| {
                FilteredSequencerBlockError::incorrect_rollup_transactions_root_length(e.len())
            })?;

        let all_rollup_ids: Vec<RollupId> = all_rollup_ids
            .into_iter()
            .map(RollupId::try_from_vec)
            .collect::<Result<_, _>>()
            .map_err(FilteredSequencerBlockError::invalid_rollup_id)?;

        if !rollup_transactions_proof.verify(&Sha256::digest(rollup_transactions_root), data_hash) {
            return Err(FilteredSequencerBlockError::rollup_transactions_not_in_sequencer_block());
        }

        if !are_rollup_ids_included(all_rollup_ids.iter().copied(), &rollup_ids_proof, data_hash) {
            return Err(FilteredSequencerBlockError::rollup_ids_not_in_sequencer_block());
        }

        Ok(Self {
            block_hash,
            header,
            rollup_transactions,
            rollup_transactions_root,
            rollup_transactions_proof,
            all_rollup_ids,
            rollup_ids_proof,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FilteredSequencerBlockError {
    #[error("the rollup ID in the raw protobuf rollup transaction was not 32 bytes long")]
    InvalidRollupId(IncorrectRollupIdLength),
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("failed to create a cometbft header from the raw protobuf header")]
    CometBftHeader(tendermint::Error),
    #[error("hashing the CometBFT block.header returned an empty hash which is not permitted")]
    CometBftBlockHashIsNone,
    #[error("failed parsing a raw protobuf rollup transaction")]
    ParseRollupTransactions(RollupTransactionsError),
    #[error(
        "the rollup transactions in the sequencer block were not included in the block's data hash"
    )]
    RollupTransactionsNotInSequencerBlock,
    #[error("the rollup IDs in the sequencer block were not included in the block's data hash")]
    RollupIdsNotInSequencerBlock,
    #[error("failed constructing a transaction proof from the raw protobuf transaction proof")]
    TransactionProofInvalid(merkle::audit::InvalidProof),
    #[error("failed constructing a rollup ID proof from the raw protobuf rollup ID proof")]
    IdProofInvalid(merkle::audit::InvalidProof),
    #[error(
        "the root derived from the rollup transactions in the sequencer block did not match the \
         root stored in the same block"
    )]
    IncorrectRollupTransactionsRootLength(usize),
}

impl FilteredSequencerBlockError {
    fn invalid_rollup_id(source: IncorrectRollupIdLength) -> Self {
        Self::InvalidRollupId(source)
    }

    fn field_not_set(field: &'static str) -> Self {
        Self::FieldNotSet(field)
    }

    fn cometbft_header(source: tendermint::Error) -> Self {
        Self::CometBftHeader(source)
    }

    fn comet_bft_block_hash_is_none() -> Self {
        Self::CometBftBlockHashIsNone
    }

    fn parse_rollup_transactions(source: RollupTransactionsError) -> Self {
        Self::ParseRollupTransactions(source)
    }

    fn rollup_transactions_not_in_sequencer_block() -> Self {
        Self::RollupTransactionsNotInSequencerBlock
    }

    fn rollup_ids_not_in_sequencer_block() -> Self {
        Self::RollupIdsNotInSequencerBlock
    }

    fn transaction_proof_invalid(source: merkle::audit::InvalidProof) -> Self {
        Self::TransactionProofInvalid(source)
    }

    fn id_proof_invalid(source: merkle::audit::InvalidProof) -> Self {
        Self::IdProofInvalid(source)
    }

    fn incorrect_rollup_transactions_root_length(len: usize) -> Self {
        Self::IncorrectRollupTransactionsRootLength(len)
    }
}

#[cfg(test)]
mod test {
    use sha2::Digest as _;

    use super::*;
    use crate::sequencer::v1alpha1::{
        merkle_leaf_from_rollup_txs,
        test_utils::make_cometbft_block,
    };

    #[test]
    fn test_sequencer_block_from_cometbft_block() {
        let block = make_cometbft_block();
        let sequencer_block = SequencerBlock::try_from_cometbft(block).unwrap();
        let rollup_ids_root =
            merkle::Tree::from_leaves(sequencer_block.rollup_transactions.keys()).root();

        let rollup_transaction_tree = derive_merkle_tree_from_rollup_txs(
            sequencer_block
                .rollup_transactions
                .iter()
                .map(|(id, txs)| (id, txs.transactions()))
                .collect::<Vec<(&RollupId, &[Vec<u8>])>>(),
        );

        for (id, rollup_transactions) in sequencer_block.rollup_transactions {
            let leaf = merkle_leaf_from_rollup_txs(&id, &rollup_transactions.transactions);
            assert!(
                rollup_transactions
                    .proof()
                    .verify(&leaf, rollup_transaction_tree.root())
            );
        }

        let data_hash: [u8; 32] = sequencer_block
            .header
            .data_hash
            .unwrap()
            .as_bytes()
            .try_into()
            .unwrap();
        assert!(
            sequencer_block
                .rollup_transactions_proof
                .verify(&Sha256::digest(rollup_transaction_tree.root()), data_hash)
        );
        assert!(
            sequencer_block
                .rollup_ids_proof
                .verify(&Sha256::digest(rollup_ids_root), data_hash)
        );
    }

    #[test]
    fn test_filtered_sequencer_block_to_from_raw() {
        let block = make_cometbft_block();
        let sequencer_block = SequencerBlock::try_from_cometbft(block).unwrap();
        let rollup_ids = sequencer_block
            .rollup_transactions
            .keys()
            .copied()
            .collect::<Vec<RollupId>>();
        let filtered_sequencer_block = sequencer_block.into_filtered_block(rollup_ids);

        let raw = filtered_sequencer_block.clone().into_raw();
        let from_raw = FilteredSequencerBlock::try_from_raw(raw).unwrap();

        assert_eq!(filtered_sequencer_block, from_raw);
    }
}
