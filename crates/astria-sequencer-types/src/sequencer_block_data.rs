use std::{
    cmp::Ordering,
    collections::HashMap,
};

use base64::{
    engine::general_purpose,
    Engine as _,
};
use bincode;
use eyre::{
    bail,
    ensure,
    eyre,
    WrapErr as _,
};
use serde::{
    Deserialize,
    Serialize,
};
use tendermint::{
    block::{
        Commit,
        Header,
        Height,
    },
    Block,
    Hash,
};
use thiserror::Error;
use tracing::debug;

use crate::namespace::Namespace;

#[derive(Error, Debug)]
pub enum Error {
    #[error("block has no data hash")]
    MissingDataHash,
}

/// `SequencerBlockData` represents a sequencer block's data
/// to be submitted to the DA layer.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct SequencerBlockData {
    pub(crate) block_hash: Hash,
    pub(crate) header: Header,
    /// This field should be set for every block with height > 1.
    pub(crate) last_commit: Option<Commit>,
    /// namespace -> rollup txs
    pub(crate) rollup_txs: HashMap<Namespace, Vec<Vec<u8>>>,
}

impl Ord for SequencerBlockData {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.header.height.cmp(&other.header.height) {
            Ordering::Equal => other.header.time.cmp(&self.header.time),
            other => other,
        }
    }
}

impl PartialOrd for SequencerBlockData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for SequencerBlockData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.block_hash.hash(state);
        self.header.hash().hash(state);
        if let Some(commit) = self.last_commit.clone() {
            let encoded: Vec<u8> = bincode::serialize(&commit).unwrap();
            encoded.hash(state);
        }

        let mut txs_keys: Vec<&Namespace> = self.rollup_txs.keys().collect();
        txs_keys.sort();
        for key in txs_keys {
            key.hash(state);
            self.rollup_txs.get(key).hash(state);
        }
    }
}

impl SequencerBlockData {
    /// Creates a new `SequencerBlockData` from the given data.
    ///
    /// # Errors
    ///
    /// - if the block hash does not correspond to the hashed header provided
    pub fn new(
        block_hash: Hash,
        header: Header,
        last_commit: Option<Commit>,
        rollup_txs: HashMap<Namespace, Vec<Vec<u8>>>,
    ) -> eyre::Result<Self> {
        let data = Self {
            block_hash,
            header,
            last_commit,
            rollup_txs,
        };
        data.verify_block_hash()?;
        // TODO: also verify last_commit_hash
        Ok(data)
    }

    #[must_use]
    pub fn block_hash(&self) -> Hash {
        self.block_hash
    }

    #[must_use]
    pub fn header(&self) -> &Header {
        &self.header
    }

    #[must_use]
    pub fn last_commit(&self) -> &Option<Commit> {
        &self.last_commit
    }

    #[must_use]
    pub fn rollup_txs(&self) -> &HashMap<Namespace, Vec<Vec<u8>>> {
        &self.rollup_txs
    }

    #[allow(clippy::type_complexity)]
    #[must_use]
    pub fn into_values(
        self,
    ) -> (
        Hash,
        Header,
        Option<Commit>,
        HashMap<Namespace, Vec<Vec<u8>>>,
    ) {
        (
            self.block_hash,
            self.header,
            self.last_commit,
            self.rollup_txs,
        )
    }

    /// Converts the `SequencerBlockData` into bytes using json.
    ///
    /// # Errors
    ///
    /// - if the data cannot be serialized into json
    pub fn to_bytes(&self) -> eyre::Result<Vec<u8>> {
        // TODO: don't use json, use our own serializer (or protobuf for now?)
        serde_json::to_vec(self).wrap_err("failed serializing signed namespace data to json")
    }

    /// Converts json-encoded bytes into a `SequencerBlockData`.
    ///
    /// # Errors
    ///
    /// - if the data cannot be deserialized from json
    /// - if the block hash cannot be verified
    pub fn from_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        let data: Self = serde_json::from_slice(bytes)
            .wrap_err("failed deserializing signed namespace data from bytes")?;
        data.verify_block_hash()
            .wrap_err("failed to verify block hash")?;
        Ok(data)
    }

    /// Converts a Tendermint block into a `SequencerBlockData`.
    /// it parses the block for `SequenceAction`s and namespaces them accordingly.
    ///
    /// # Errors
    ///
    /// - if the block has no data hash
    /// - if a transaction in the block cannot be parsed
    pub fn from_tendermint_block(b: Block) -> eyre::Result<Self> {
        use astria_sequencer::transaction::Signed;

        if b.header.data_hash.is_none() {
            bail!(Error::MissingDataHash);
        }

        // we unwrap sequencer txs into rollup-specific data here,
        // and namespace them correspondingly
        let mut rollup_txs = HashMap::new();

        for (index, tx) in b.data.iter().enumerate() {
            debug!(
                index,
                bytes = general_purpose::STANDARD.encode(tx.as_slice()),
                "parsing data from tendermint block",
            );

            let tx = Signed::try_from_slice(tx)
                .map_err(|e| eyre!(e))
                .wrap_err("failed reading signed sequencer transaction from bytes")?;
            tx.transaction().actions().iter().for_each(|action| {
                if let Some(action) = action.as_sequence() {
                    let namespace = Namespace::from_slice(action.chain_id());
                    let txs = rollup_txs.entry(namespace).or_insert(vec![]);
                    txs.push(action.data().to_vec());
                }
            });
        }

        let data = Self {
            block_hash: b.header.hash(),
            header: b.header,
            last_commit: b.last_commit,
            rollup_txs,
        };
        Ok(data)
    }

    /// verifies that the merkle root of the tree consisting of the block header
    /// matches the block's hash.
    ///
    /// # Errors
    ///
    /// - if the block hash calculated from the header does not match the block hash stored
    ///  in the sequencer block
    fn verify_block_hash(&self) -> eyre::Result<()> {
        let block_hash = self.header.hash();
        ensure!(
            block_hash == self.block_hash,
            "block hash calculated from tendermint header does not match block hash stored in \
             sequencer block",
        );
        Ok(())
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
            self.height().value() > 0,
            "block height must be greater than 0"
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
    use std::collections::HashMap;

    use super::SequencerBlockData;
    use crate::DEFAULT_NAMESPACE;

    #[test]
    fn sequencer_block_to_bytes() {
        let header = crate::test_utils::default_header();
        let block_hash = header.hash();
        let mut expected = SequencerBlockData {
            block_hash,
            header,
            last_commit: None,
            rollup_txs: HashMap::new(),
        };
        expected
            .rollup_txs
            .insert(DEFAULT_NAMESPACE, vec![vec![0x44, 0x55, 0x66]]);

        let bytes = expected.to_bytes().unwrap();
        let actual = SequencerBlockData::from_bytes(&bytes).unwrap();
        assert_eq!(expected, actual);
    }
}
