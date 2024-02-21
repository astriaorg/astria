use indexmap::IndexMap;
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
        asset,
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
}

#[derive(Debug, thiserror::Error)]
pub enum RollupTransactionsErrorKind {
    #[error("`id` field is invalid")]
    RollupId(#[source] IncorrectRollupIdLength),
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
        let id = RollupId::try_from_slice(&id).map_err(RollupTransactionsError::rollup_id)?;
        Ok(Self {
            id,
            transactions,
        })
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

    fn parse_rollup_transactions(source: IncorrectRollupIdLength) -> Self {
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

    fn raw_signed_transactin_conversion(source: transaction::SignedTransactionError) -> Self {
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
#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
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
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::module_name_repetitions)]
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

    /// The height stored in this sequencer block.
    #[must_use]
    pub fn height(&self) -> tendermint::block::Height {
        self.header.height
    }

    #[must_use]
    pub fn rollup_transactions(&self) -> &IndexMap<RollupId, Vec<Vec<u8>>> {
        &self.rollup_transactions
    }

    /// Returns the map of rollup transactions, consuming `self`.
    #[must_use]
    pub fn into_rollup_transactions(self) -> IndexMap<RollupId, Vec<Vec<u8>>> {
        self.rollup_transactions
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

        let mut rollup_transactions = IndexMap::new();
        for elem in data_list {
            let raw_tx = raw::SignedTransaction::decode(&*elem)
                .map_err(SequencerBlockError::signed_transaction_prootbof_decode)?;
            let signed_tx = SignedTransaction::try_from_raw(raw_tx)
                .map_err(SequencerBlockError::raw_signed_transactin_conversion)?;
            for action in signed_tx.into_unsigned().actions {
                if let action::Action::Sequence(action::SequenceAction {
                    rollup_id,
                    data,
                    fee_asset_id: _,
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
            return Err(
                SequencerBlockError::rollup_transactions_root_does_not_match_reconstructed(),
            );
        }

        // ensure the rollup IDs commitment matches the one calculated from the rollup data
        if rollup_ids_root != merkle::Tree::from_leaves(rollup_transactions.keys()).root() {
            return Err(SequencerBlockError::rollup_ids_root_does_not_match_reconstructed());
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
    use sha2::{
        Digest as _,
        Sha256,
    };
    merkle::Tree::from_leaves(iter.into_iter().map(|item| Sha256::digest(&item)))
}

/// [`Deposit`] represents a deposit from the sequencer to a rollup.
///
/// A [`Deposit`] is constructed whenever a [`BridgeLockAction`] is executed
/// and stored as part of the block's events.
#[derive(Debug, Clone)]
pub struct Deposit {
    pub rollup_id: RollupId,
    pub amount: u128,
    pub asset_id: asset::Id,
    pub destination_chain_address: String,
}

impl Deposit {
    #[must_use]
    pub fn into_raw(self) -> raw::Deposit {
        let Self {
            rollup_id,
            amount,
            asset_id,
            destination_chain_address,
        } = self;
        raw::Deposit {
            rollup_id: rollup_id.to_vec(),
            amount: Some(amount.into()),
            asset_id: asset_id.as_bytes().to_vec(),
            destination_chain_address,
        }
    }

    #[must_use]
    pub fn try_from_raw(raw: raw::Deposit) -> Result<Self, DepositError> {
        let raw::Deposit {
            rollup_id,
            amount,
            asset_id,
            destination_chain_address,
        } = raw;
        let amount = amount.ok_or(DepositError::FieldNotSet("amount"))?.into();
        let rollup_id = RollupId::try_from_slice(&rollup_id)?;
        let asset_id = asset::Id::try_from_slice(&asset_id)?;
        Ok(Self {
            rollup_id,
            amount,
            asset_id,
            destination_chain_address,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DepositError {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error(transparent)]
    IncorrectRollupIdLength(#[from] IncorrectRollupIdLength),
    #[error(transparent)]
    IncorrectAssetIdLength(#[from] asset::IncorrectAssetIdLength),
}
