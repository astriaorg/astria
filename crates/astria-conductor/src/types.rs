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
