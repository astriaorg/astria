use crate::{
    generated::sequencer::v1alpha1 as raw,
    native::{
        sequencer::v1alpha1::validation::{
            InclusionProof,
            InclusionProofError,
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
    ChainIdsCommitmentNotGenerated,
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
    LastCommitHashNotGenerated,
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
pub struct SequencerBlock {
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

impl SequencerBlock {
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

        let data_hash = header
            .data_hash
            .ok_or(SequencerBlockError::HeaderDataHashNotSet)?;

        if block_hash != header.hash().as_bytes() {
            return Err(SequencerBlockError::BlockHashNotHeaderHash);
        }

        let Ok::<[u8; 32], _>(action_tree_root) = action_tree_root.try_into() else {
            return Err(SequencerBlockError::ActionTreeRootNot32Bytes);
        };

        let Some(action_tree_inclusion_proof) = action_tree_inclusion_proof else {
            return Err(SequencerBlockError::ActionTreeInclusionProofNotSet);
        };

        let action_tree_inclusion_proof = InclusionProof::try_from_raw(action_tree_inclusion_proof)
            .map_err(SequencerBlockError::InclusionProofConversion)?;

        action_tree_inclusion_proof
            .verify(&action_tree_root, data_hash)
            .map_err(SequencerBlockError::ActionTreeRootVerification)?;

        let Ok(chain_ids_commitment) = chain_ids_commitment.try_into() else {
            return Err(SequencerBlockError::ChainIdsCommitmentNot32Bytes);
        };

        let rollup_transactions = rollup_transactions
            .into_iter()
            .map(RollupTransactions::try_from_raw)
            .collect::<Result<Vec<_>, _>>()
            .map_err(SequencerBlockError::RollupTransactionsConversion)?;

        let generated_commitment = sequencer_validation::utils::generate_commitment(
            rollup_transactions.iter().map(|tx| &tx.chain_id[..]),
        );
        if chain_ids_commitment != generated_commitment {
            return Err(SequencerBlockError::ChainIdsCommitmentNotGenerated);
        }

        // genesis and height 1 do not have a last commit
        if header.height.value() <= 1 {
            return Ok(Self {
                block_hash,
                header,
                last_commit: None,
                rollup_transactions,
                action_tree_root,
                action_tree_inclusion_proof,
                chain_ids_commitment,
            });
        }
        // calculate and verify last_commit_hash
        let last_commit = {
            let Some(hash) = header.last_commit_hash else {
                return Err(SequencerBlockError::LastCommitHashNotSet);
            };
            let Some(commit) = last_commit else {
                return Err(SequencerBlockError::LastCommitNotSet);
            };
            if hash != calculate_last_commit_hash(&commit) {
                return Err(SequencerBlockError::LastCommitHashNotGenerated)?;
            }
            tendermint::block::Commit::try_from(commit)
                .map_err(SequencerBlockError::LastCommitConversion)?
        };

        Ok(Self {
            block_hash,
            header,
            last_commit: Some(last_commit),
            rollup_transactions,
            action_tree_root,
            action_tree_inclusion_proof,
            chain_ids_commitment,
        })
    }
}

#[must_use]
fn calculate_last_commit_hash(commit: &tendermint_proto::types::Commit) -> tendermint::Hash {
    use tendermint::{
        crypto,
        merkle,
    };

    let signatures = commit
        .signatures
        .iter()
        .map(prost::Message::encode_to_vec)
        .collect::<Vec<_>>();
    tendermint::Hash::Sha256(merkle::simple_hash_from_byte_vectors::<
        crypto::default::Sha256,
    >(&signatures))
}
