use prost::Message;
use sequencer_validation::MerkleTree;

use super::{
    SignedTransaction,
    SignedTransactionError,
};
use crate::{
    generated::sequencer::v1alpha1 as raw,
    native::{
        sequencer::v1alpha1::{
            validation::{
                InclusionProof,
                InclusionProofError,
            },
            Action,
            SequenceAction,
        },
        Protobuf as _,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum RollupTransactionsError {
    #[error("the `chain_id` field of the raw protobuf message was not 32 bytes long")]
    ChainIdNot32Bytes,
}

#[derive(Clone)]
pub struct RollupTransactions {
    /// Opaque bytes identifying the rollup that these transactions belong to.
    pub chain_id: [u8; 32],
    /// The serialized opaque bytes of the rollup transactions.
    pub transactions: Vec<Vec<u8>>,
}

impl RollupTransactions {
    pub fn try_from_raw(raw: raw::RollupTransactions) -> Result<Self, RollupTransactionsError> {
        let raw::RollupTransactions {
            chain_id,
            transactions,
        } = raw;
        let Ok(chain_id) = chain_id.try_into() else {
            return Err(RollupTransactionsError::ChainIdNot32Bytes);
        };
        Ok(Self {
            chain_id,
            transactions,
        })
    }

    pub fn into_raw(self) -> raw::RollupTransactions {
        let Self {
            chain_id,
            transactions,
        } = self;
        raw::RollupTransactions {
            chain_id: chain_id.into(),
            transactions,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SequencerBlockError {
    #[error(
        "failed to verify data hash in cometbft header against action tree root and inclusion \
         proof root in sequencer block body"
    )]
    ActionTreeRootVerification(#[source] sequencer_validation::VerificationFailure),
    #[error("the `action_tree_inclusion_proof` field of the raw sequencer block was not set")]
    ActionTreeInclusionProofNotSet,
    #[error("the action tree root contained in the raw sequencer block was not 32 bytes long")]
    ActionTreeRootNot32Bytes,
    #[error("the block hash contained in the raw sequencer block was not 32 bytes long")]
    BlockHashNot32Bytes,
    #[error(
        "the block hash contained in the raw sequencer block did not match the hash calculated \
         from the cometbft header"
    )]
    BlockHashNotHeaderHash,
    #[error(
        "the `chain_id_commitments` field in the raw sequencer block does not match match the \
         commitment generated from the chain IDs recorded in the `rollup_transactions` field"
    )]
    ChainIdsCommitmentWrong,
    #[error("the chain ID commitiment contained in the raw sequencer block was not 32 bytes long")]
    ChainIdsCommitmentNot32Bytes,
    #[error("the `header` field in the raw sequencer block was not set")]
    HeaderNotSet,
    #[error("failed converting the raw protobuf `header` field to a cometbft header")]
    HeaderConversion(#[source] tendermint::Error),
    #[error("the cometbft header did not contain a data hash, but it must be set")]
    HeaderDataHashNotSet,
    #[error(
        "failed converting inclusion proof in raw sequencer block to an astria inclusion proof"
    )]
    InclusionProofConversion(#[source] InclusionProofError),
    #[error(
        "the last commit hash contained in the raw sequencer block does not match match the hash \
         generated from the last commit signatures"
    )]
    LastCommitHashWrong,
    #[error(
        "the `header.last_commit_hash` field in the raw sequencer block was not set, even though \
         the height was above 1"
    )]
    LastCommitHashNotSet,
    #[error(
        "failed converting `last_commit` field in the raw sequencer block to a common tendermint \
         commit"
    )]
    LastCommitConversion(#[source] tendermint::Error),
    #[error(
        "the `last_commit` field in the raw sequencer block was not set, even though the height \
         was above 1"
    )]
    LastCommitNotSet,
    #[error(
        "failed converting one item in the `rollup_transactions` field in the raw sequencer block \
         to a common rollup transaction"
    )]
    RollupTransactionsConversion(#[source] RollupTransactionsError),
}

#[derive(Clone)]
pub struct UnverifiedSequencerBlock {
    /// The hash of the sequencer block.
    pub block_hash: [u8; 32],
    /// The original cometbft header that was the input to this sequencer block.
    pub header: tendermint::block::Header,
    /// The commit/set of signatures that commited this block.
    pub last_commit: Option<tendermint::block::Commit>,
    /// The collection of rollup transactions that were included in this block.
    pub rollup_transactions: Vec<RollupTransactions>,
    /// The root of the action tree of this block. Must be 32 bytes.
    pub action_tree_root: [u8; 32],
    /// The proof that the action tree root was included in `header.data_hash`.
    pub action_tree_inclusion_proof: InclusionProof,
    /// The root of the merkle tree constructed form the chain IDs of the rollup
    /// transactions in this block.
    pub chain_ids_commitment: [u8; 32],
}

impl UnverifiedSequencerBlock {
    pub fn try_from_raw(raw: raw::SequencerBlock) -> Result<Self, SequencerBlockError> {
        let raw::SequencerBlock {
            block_hash,
            header,
            last_commit,
            rollup_transactions,
            action_tree_root,
            action_tree_inclusion_proof,
            chain_ids_commitment,
        } = raw;

        let block_hash: [u8; 32] = block_hash
            .try_into()
            .map_err(|_| SequencerBlockError::BlockHashNot32Bytes)?;

        let header: tendermint::block::Header = 'conversion: {
            let Some(header) = header else {
                break 'conversion Err(SequencerBlockError::HeaderNotSet);
            };
            header
                .try_into()
                .map_err(SequencerBlockError::HeaderConversion)
        }?;

        let Ok::<[u8; 32], _>(action_tree_root) = action_tree_root.try_into() else {
            return Err(SequencerBlockError::ActionTreeRootNot32Bytes);
        };

        let Some(action_tree_inclusion_proof) = action_tree_inclusion_proof else {
            return Err(SequencerBlockError::ActionTreeInclusionProofNotSet);
        };

        let action_tree_inclusion_proof = InclusionProof::try_from_raw(action_tree_inclusion_proof)
            .map_err(SequencerBlockError::InclusionProofConversion)?;

        let Ok(chain_ids_commitment) = chain_ids_commitment.try_into() else {
            return Err(SequencerBlockError::ChainIdsCommitmentNot32Bytes);
        };

        let rollup_transactions = rollup_transactions
            .into_iter()
            .map(RollupTransactions::try_from_raw)
            .collect::<Result<Vec<_>, _>>()
            .map_err(SequencerBlockError::RollupTransactionsConversion)?;

        // calculate and verify last_commit_hash
        let last_commit = last_commit
            .map(tendermint::block::Commit::try_from)
            .transpose()
            .map_err(SequencerBlockError::LastCommitConversion)?;

        Ok(Self {
            block_hash,
            header,
            last_commit,
            rollup_transactions,
            action_tree_root,
            action_tree_inclusion_proof,
            chain_ids_commitment,
        })
    }

    pub fn into_verified(self) -> Result<SequencerBlock, SequencerBlockError> {
        let Self {
            block_hash,
            header,
            last_commit,
            rollup_transactions,
            action_tree_root,
            action_tree_inclusion_proof,
            chain_ids_commitment,
        } = self;

        let data_hash = header
            .data_hash
            .ok_or(SequencerBlockError::HeaderDataHashNotSet)?;

        if block_hash != header.hash().as_bytes() {
            return Err(SequencerBlockError::BlockHashNotHeaderHash);
        }

        action_tree_inclusion_proof
            .verify(&action_tree_root, data_hash)
            .map_err(SequencerBlockError::ActionTreeRootVerification)?;

        let generated_commitment = sequencer_validation::utils::generate_commitment(
            rollup_transactions.iter().map(|tx| &tx.chain_id[..]),
        );
        if chain_ids_commitment != generated_commitment {
            return Err(SequencerBlockError::ChainIdsCommitmentWrong);
        }

        // genesis and height 1 do not have a last commit
        if header.height.value() <= 1 {
            return Ok(SequencerBlock {
                block_hash,
                header,
                last_commit: None,
                rollup_transactions,
                action_tree_root,
                action_tree_inclusion_proof,
                chain_ids_commitment,
            });
        }
        // calculate and verify last_commit_hash. Both of these must be set
        // if height > 1
        let Some(hash) = header.last_commit_hash else {
            return Err(SequencerBlockError::LastCommitHashNotSet);
        };
        let Some(commit) = last_commit else {
            return Err(SequencerBlockError::LastCommitNotSet);
        };

        if hash != calculate_last_commit_hash(&commit) {
            return Err(SequencerBlockError::LastCommitHashWrong)?;
        }

        Ok(SequencerBlock {
            block_hash,
            header,
            last_commit: Some(commit),
            rollup_transactions,
            action_tree_root,
            action_tree_inclusion_proof,
            chain_ids_commitment,
        })
    }

    pub fn into_verified_unchecked(self) -> SequencerBlock {
        let Self {
            block_hash,
            header,
            last_commit,
            rollup_transactions,
            action_tree_root,
            action_tree_inclusion_proof,
            chain_ids_commitment,
        } = self;

        SequencerBlock {
            block_hash,
            header,
            last_commit,
            rollup_transactions,
            action_tree_root,
            action_tree_inclusion_proof,
            chain_ids_commitment,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CometBftConversionError {
    #[error("the 1st element in the cometbft datas list exists, but was not 32 bytes long")]
    ActionTreeRootNot32Bytes,
    #[error("the 2nd element in the cometbft datas list exists, but was not 32 bytes long")]
    ChainIdsCommitmentNot32Bytes,
    #[error(
        "the chain IDs commitment constructed from the chain IDs in the signed transactions \
         deserialized from the datas list does not match the 2nd element of the datas field"
    )]
    ChainIdsCommitmentWrong,
    #[error(
        "the `header.data_hash` field in the cometbft block does not match the hash generated \
         from its `datas` field"
    )]
    DataHashWrong,
    #[error(
        "failed constructing the inclusion proof for the root of the data list in the cometbft \
         header"
    )]
    InclusionProof(#[source] sequencer_validation::IndexOutOfBounds),
    #[error("the `header.data_hash` field in the cometbft block was not set")]
    MissingDataHash,
    #[error(
        "the 1st element in the cometbft datas list is the action tree root, but the list was \
         shorter than that"
    )]
    NoActionTreeRoot,
    #[error(
        "the 2nd element in the cometbft datas list is the chain IDs commitment, but the list was \
         shorter than that"
    )]
    NoChainIdsCommitment,
    #[error(
        "failed converting a raw protobuf signed transaction to a common astria signed transaction"
    )]
    RawSignedTransactionConversion(#[source] SignedTransactionError),
    #[error("failed decoding a cometbft datas element into a protobuf raw signed transaction")]
    SignedTransactionProtobufDecode(#[source] prost::DecodeError),
}

#[derive(Clone)]
pub struct SequencerBlock {
    /// The hash of the sequencer block.
    block_hash: [u8; 32],
    /// The original cometbft header that was the input to this sequencer block.
    header: tendermint::block::Header,
    /// The commit/set of signatures that commited this block.
    last_commit: Option<tendermint::block::Commit>,
    /// The collection of rollup transactions that were included in this block.
    rollup_transactions: Vec<RollupTransactions>,
    /// The root of the action tree of this block. Must be 32 bytes.
    action_tree_root: [u8; 32],
    /// The proof that the action tree root was included in `header.data_hash`.
    action_tree_inclusion_proof: InclusionProof,
    /// The root of the merkle tree constructed form the chain IDs of the rollup
    /// transactions in this block.
    chain_ids_commitment: [u8; 32],
}

impl SequencerBlock {
    pub fn into_unverified(self) -> UnverifiedSequencerBlock {
        let Self {
            block_hash,
            header,
            last_commit,
            rollup_transactions,
            action_tree_root,
            action_tree_inclusion_proof,
            chain_ids_commitment,
        } = self;
        UnverifiedSequencerBlock {
            block_hash,
            header,
            last_commit,
            rollup_transactions,
            action_tree_root,
            action_tree_inclusion_proof,
            chain_ids_commitment,
        }
    }

    /// Converts a Tendermint block into a `SequencerBlockData`.
    /// it parses the block for `SequenceAction`s and namespaces them accordingly.
    ///
    /// # Errors
    ///
    /// - if the block has no data hash
    /// - if the block has no transactions
    /// - if the block's first transaction is not the 32-byte action tree root
    /// - if a transaction in the block cannot be parsed
    /// - if the block's `data_hash` does not match the one calculated from the transactions
    /// - if the inclusion proof of the action tree root in the block's `data_hash` cannot be
    ///   generated
    ///
    /// See `specs/sequencer-inclusion-proofs.md` for most details on the action tree root
    /// and inclusion proof purpose.
    pub fn try_from_cometbft(
        block: tendermint::block::Block,
    ) -> Result<Self, CometBftConversionError> {
        let Some(data_hash) = block.header.data_hash else {
            return Err(CometBftConversionError::MissingDataHash);
        };

        let mut datas = block.data.iter();

        // The first entry is the action tree root
        let action_tree_root: [u8; 32] = datas
            .next()
            .map(Vec::as_slice)
            .ok_or(CometBftConversionError::NoActionTreeRoot)?
            .try_into()
            .map_err(|_| CometBftConversionError::ActionTreeRootNot32Bytes)?;

        // The second entry is the chain IDs commitment
        let chain_ids_commitment: [u8; 32] = datas
            .next()
            .map(Vec::as_slice)
            .ok_or(CometBftConversionError::NoChainIdsCommitment)?
            .try_into()
            .map_err(|_| CometBftConversionError::ChainIdsCommitmentNot32Bytes)?;

        // The remaining bytes are the rollup transactions
        let mut rollup_map = indexmap::IndexMap::<_, Vec<_>>::new();
        for tx_bytes in datas {
            let raw_tx = raw::SignedTransaction::decode(&**tx_bytes)
                .map_err(CometBftConversionError::SignedTransactionProtobufDecode)?;
            let tx = SignedTransaction::try_from_raw(raw_tx)
                .map_err(CometBftConversionError::RawSignedTransactionConversion)?;

            for action in tx.into_unsigned_transaction().actions {
                if let Action::Sequence(seq) = action {
                    let SequenceAction {
                        chain_id,
                        data,
                    } = seq;
                    if let Some(datas) = rollup_map.get_mut(&chain_id) {
                        datas.push(data)
                    } else {
                        rollup_map.insert(chain_id, vec![data]);
                    }
                }
            }
        }

        // generate the action tree root proof of inclusion in `Header::data_hash`
        let data_tree = calculate_merkle_tree_from_cometbft_data(&block.data);
        if data_tree.root() != data_hash.as_bytes() {
            return Err(CometBftConversionError::DataHashWrong);
        }
        let action_tree_inclusion_proof = data_tree
            .prove_inclusion(0) // action tree root is always the first tx in a block
            .map_err(CometBftConversionError::InclusionProof)?;

        // ensure the chain IDs commitment matches the one calculated from the rollup data
        rollup_map.sort_keys();
        let chain_ids = rollup_map
            .keys()
            .copied()
            .map(Into::into)
            .collect::<Vec<_>>();
        let calculated_chain_ids_commitment = MerkleTree::from_leaves(chain_ids).root();
        if calculated_chain_ids_commitment != chain_ids_commitment {
            return Err(CometBftConversionError::ChainIdsCommitmentWrong);
        }
        let rollup_transactions = rollup_map
            .into_iter()
            .map(|(chain_id, transactions)| RollupTransactions {
                chain_id,
                transactions,
            })
            .collect();

        let tendermint::hash::Hash::Sha256(block_hash) = block.header.hash() else {
            panic!(
                "Header::hash is guaranteed to produce the Sha256 variant of the `Hash` type. If \
                 that has changed then that is a breaking change. Please report."
            );
        };
        Ok(Self {
            block_hash,
            header: block.header,
            last_commit: block.last_commit,
            rollup_transactions,
            action_tree_root,
            action_tree_inclusion_proof,
            chain_ids_commitment,
        })
    }

    pub fn try_from_raw(raw: raw::SequencerBlock) -> Result<Self, SequencerBlockError> {
        UnverifiedSequencerBlock::try_from_raw(raw)
            .and_then(UnverifiedSequencerBlock::into_verified)
    }
}

fn calculate_merkle_tree_from_cometbft_data(txs: &[Vec<u8>]) -> MerkleTree {
    let hashed_txs = txs
        .into_iter()
        .map(|tx| sequencer_validation::utils::sha256_hash(tx).into())
        .collect::<Vec<_>>();
    MerkleTree::from_leaves(hashed_txs)
}

#[must_use]
fn calculate_last_commit_hash(commit: &tendermint::block::Commit) -> tendermint::Hash {
    use prost::Message as _;
    use tendermint::{
        crypto,
        merkle,
    };

    let signatures = commit
        .signatures
        .iter()
        .map(|sig| tendermint_proto::types::CommitSig::from(sig.clone()).encode_to_vec())
        .collect::<Vec<_>>();
    tendermint::Hash::Sha256(merkle::simple_hash_from_byte_vectors::<
        crypto::default::Sha256,
    >(&signatures))
}
