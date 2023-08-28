use std::cmp::Ordering;

use astria_sequencer_types::{
    Namespace,
    RawSequencerBlockData,
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

        let our_rollup_data = rollup_data.remove(&namespace).unwrap_or_default();
        Self {
            block_hash,
            header,
            rollup_transactions: our_rollup_data.transactions,
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
    #[must_use]
    pub fn height(&self) -> Height {
        self.header().height
    }

    /// Get the height of the block's parent.
    ///
    /// # Panics
    ///
    /// This function will panic if the block height is less than or equal to 1.
    /// Only the genesis block has a height of 1, and all other blocks must have
    /// a larger height.
    #[must_use]
    pub fn parent_height(&self) -> Height {
        assert!(
            self.height().value() > 1,
            "block height must be greater than 1"
        );
        Height::try_from(self.header().height.value() - 1)
            .expect("should have been able to decriment tendermint height")
    }

    /// Get the height of the block's child, or the next block.
    #[must_use]
    pub fn child_height(&self) -> Height {
        self.height().increment()
    }

    /// Get the hash of the block's parent.
    ///
    /// Will return `Some(Hash)` if the block has a parent hash.
    /// Will return `None` if the block does not have a parent hash. This is the case for the
    /// genesis block.
    #[must_use]
    pub fn parent_hash(&self) -> Option<Hash> {
        if let Some(parent_hash) = self.header().last_block_id {
            return Some(parent_hash.hash);
        }
        None
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{
            Hash,
            Hasher,
        },
    };

    use sha2::Digest as _;
    use tendermint::{
        block::Id as BlockId,
        hash::Hash as THash,
    };

    use super::*;

    fn hash(s: &[u8]) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(s);
        hasher.finalize().to_vec()
    }

    fn get_test_block_subset() -> SequencerBlockSubset {
        SequencerBlockSubset {
            block_hash: hash(b"block1").try_into().unwrap(),
            header: astria_sequencer_types::test_utils::default_header(),
            rollup_transactions: vec![],
        }
    }

    // build a vec of sequential blocks for testing
    fn get_test_block_vec(num_blocks: u32) -> Vec<SequencerBlockSubset> {
        // let namespace = Namespace::from_slice(b"test");

        let mut block = get_test_block_subset();
        block.rollup_transactions.push(b"test_transaction".to_vec());

        let mut blocks = vec![];

        block.header.height = 1_u32.into();
        blocks.push(block);

        for i in 2..=num_blocks {
            let current_hash_string = String::from("block") + &i.to_string();
            let prev_hash_string = String::from("block") + &(i - 1).to_string();
            let current_byte_hash: &[u8] = &current_hash_string.into_bytes();
            let prev_byte_hash: &[u8] = &prev_hash_string.into_bytes();

            let mut block = get_test_block_subset();
            block.block_hash = THash::try_from(hash(current_byte_hash)).unwrap();
            block.rollup_transactions.push(b"test_transaction".to_vec());

            block.header.height = i.into();
            let block_id = BlockId {
                hash: THash::try_from(hash(prev_byte_hash)).unwrap(),
                ..Default::default()
            };
            block.header.last_block_id = Some(block_id);

            blocks.push(block);
        }
        blocks
    }

    // test that SequencerBlockSubset can be sorted
    #[tokio::test]
    async fn sequencer_block_data_ordering() {
        let blocks = get_test_block_vec(3);
        // build the vec in reverse order
        let mut blocks_sorted = vec![blocks[2].clone(), blocks[1].clone(), blocks[0].clone()];
        blocks_sorted.sort();
        assert_eq!(blocks_sorted, blocks);
    }

    // test that the invariant k1 == k2 -> hash(k1) == hash(k2) holds for SequencerBlockSubset
    #[tokio::test]
    async fn sequencer_block_data_hashing_invariant() {
        let blocks = get_test_block_vec(1);
        let block1a = blocks[0].clone();
        let block1b = blocks[0].clone();
        assert_eq!(block1a, block1b);

        let mut hasher1 = DefaultHasher::new();
        block1a.hash(&mut hasher1);
        let block1a_hash = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        block1b.hash(&mut hasher2);
        let block1b_hash = hasher2.finish();

        assert_eq!(block1a_hash, block1b_hash);
    }
}
