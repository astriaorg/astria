use astria_proto::generated::execution::v1alpha2::{
    Block,
    CommitmentState,
};
use astria_sequencer_types::{
    ChainId,
    RawSequencerBlockData,
    SequencerBlockData,
};
use tendermint::{
    block::Header,
    Hash,
};

/// `SequencerBlockSubset` is a subset of a SequencerBlock that contains
/// information required for transaction data verification, and the transactions
/// for one specific rollup.
#[derive(Clone, Debug)]
pub(crate) struct SequencerBlockSubset {
    pub(crate) block_hash: Hash,
    pub(crate) header: Header,
    pub(crate) rollup_transactions: Vec<Vec<u8>>,
}

impl SequencerBlockSubset {
    pub(crate) fn from_sequencer_block_data(data: SequencerBlockData, chain_id: &ChainId) -> Self {
        // we don't need to verify the action tree root here,
        // as [`SequencerBlockData`] would not be constructable
        // if it was invalid
        let RawSequencerBlockData {
            block_hash,
            header,
            last_commit: _,
            mut rollup_data,
            ..
        } = data.into_raw();

        let rollup_transactions = rollup_data.remove(chain_id).unwrap_or_default();

        Self {
            block_hash,
            header,
            rollup_transactions,
        }
    }
}

/// `ExecutorCommitmentState` is a struct that contains the firm and soft Blocks from the execution
/// client. This is a utility type to avoid dealing with Option<Block>s all over the place.
#[derive(Clone, Debug)]
pub(crate) struct ExecutorCommitmentState {
    pub(crate) firm: Block,
    pub(crate) soft: Block,
}

impl ExecutorCommitmentState {
    /// Creates a new `ExecutorCommitmentState` from a `CommitmentState`.
    /// `firm` and `soft` should never be `None`
    pub(crate) fn from_execution_client_commitment_state(data: CommitmentState) -> Self {
        let firm = data.firm.expect(
            "could not convert from CommitmentState to ExecutorCommitmentState. `firm` is None. \
             This should never happen.",
        );
        let soft = data.soft.expect(
            "could not convert from CommitmentState to ExecutorCommitmentState. `soft` is None. \
             This should never happen.",
        );

        Self {
            firm,
            soft,
        }
    }
}
