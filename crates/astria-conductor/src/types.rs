use astria_sequencer_types::{
    Namespace,
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
pub struct SequencerBlockSubset {
    pub(crate) block_hash: Hash,
    pub(crate) header: Header,
    pub(crate) rollup_transactions: Vec<Vec<u8>>,
}

impl SequencerBlockSubset {
    pub(crate) fn from_sequencer_block_data(
        data: SequencerBlockData,
        namespace: Namespace,
    ) -> Option<Self> {
        // we don't need to verify the action tree root here,
        // as [`SequencerBlockData`] would not be constructable
        // if it was invalid

        let (block_hash, header, _, mut rollup_txs, ..) = data.into_values();

        let Some(rollup_data) = rollup_txs.remove(&namespace) else {
            return None;
        };

        Some(Self {
            block_hash,
            header,
            rollup_transactions: rollup_data.transactions,
        })
    }
}
