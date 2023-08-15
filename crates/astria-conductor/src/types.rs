use astria_sequencer_types::{
    Namespace,
    SequencerBlockData,
};
use tendermint::block::Header;

/// `SequencerBlockSubset` is a subset of a SequencerBlock that contains
/// information required for transaction data verification, and the transactions
/// for one specific rollup.
#[derive(Clone, Debug)]
pub struct SequencerBlockSubset {
    pub(crate) block_hash: Vec<u8>,
    pub(crate) header: Header,
    pub(crate) rollup_transactions: Vec<Vec<u8>>,
}

impl SequencerBlockSubset {
    pub(crate) fn from_sequencer_block_data(
        data: SequencerBlockData,
        namespace: Namespace,
    ) -> Self {
        let (block_hash, header, _, mut rollup_txs) = data.take_values();
        let rollup_transactions = rollup_txs.remove(&namespace).unwrap_or_default();
        Self {
            block_hash,
            header,
            rollup_transactions,
        }
    }
}
