use std::cmp::Ordering;

use astria_sequencer_types::{
    Namespace,
    SequencerBlockData,
};
use tendermint::{
    block::{
        Header,
        Height,
    },
    Hash,
};

/// `SequencerBlockSubset` is a subset of a SequencerBlock that contains
/// information required for transaction data verification, and the transactions
/// for one specific rollup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequencerBlockSubset {
    pub(crate) block_hash: Hash,
    pub(crate) header: Header,
    pub(crate) rollup_transactions: Vec<Vec<u8>>,
}

impl Ord for SequencerBlockSubset {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.header.height.cmp(&other.header.height) {
            Ordering::Equal => other.header.time.cmp(&self.header.time),
            other => other,
        }
    }
}

impl PartialOrd for SequencerBlockSubset {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// TODO: write a text to check this for the hash invariant
impl std::hash::Hash for SequencerBlockSubset {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.block_hash.hash(state);
        self.header.hash().hash(state);

        let mut transactions = self.rollup_transactions.clone();
        transactions.sort();
        for tx in transactions {
            tx.hash(state);
        }
    }
}

impl SequencerBlockSubset {
    pub(crate) fn from_sequencer_block_data(
        data: SequencerBlockData,
        namespace: Namespace,
    ) -> Self {
        let (block_hash, header, _, mut rollup_txs) = data.into_values();
        let rollup_transactions = rollup_txs.remove(&namespace).unwrap_or_default();
        Self {
            block_hash,
            header,
            rollup_transactions,
        }
    }

    /// Return the block hash.
    pub fn block_hash(&self) -> Hash {
        self.block_hash
    }

    /// Return the header of the block.
    pub fn header(&self) -> Header {
        self.header.clone()
    }

    /// Get the height of the block.
    pub fn height(&self) -> Height {
        self.header().height
    }

    /// Get the height of the block's parent.
    pub fn parent_height(&self) -> Height {
        assert!(
            self.height().value() > 1,
            "block height must be greater than 1"
        );
        Height::try_from(self.header().height.value() - 1)
            .expect("should have been able to decriment tendermint height")
    }

    /// Get the height of the block's child, or the next block.
    pub fn child_height(&self) -> Height {
        self.height().increment()
    }

    /// Get the hash of the block's parent.
    ///
    /// Will return `Some(Hash)` if the block has a parent hash.
    /// Will return `None` if the block does not have a parent hash. This is the case for the
    /// genesis block.
    pub fn parent_hash(&self) -> Option<Hash> {
        if let Some(parent_hash) = self.header().last_block_id {
            return Some(parent_hash.hash);
        }
        None
    }
}
