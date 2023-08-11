use astria_sequencer_relayer::types::{
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
        mut data: SequencerBlockData,
        namespace: Namespace,
    ) -> Self {
        let rollup_transactions = data
            .rollup_txs
            .remove(&namespace)
            .unwrap_or_default()
            .into_iter()
            .map(|tx| tx.transaction)
            .collect();
        Self {
            block_hash: data.block_hash,
            header: data.header,
            rollup_transactions,
        }
    }
}
