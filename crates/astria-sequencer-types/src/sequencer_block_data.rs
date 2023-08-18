use std::collections::HashMap;

use astria_sequencer_validation::{
    InclusionProof,
    MerkleTree,
};
use base64::{
    engine::general_purpose,
    Engine as _,
};
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

/// Rollup data that relayer/conductor need to know.
#[derive(Default, Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct RollupData {
    pub chain_id: Vec<u8>,
    pub transactions: Vec<Vec<u8>>,
}

/// `SequencerBlockData` represents a sequencer block's data
/// to be submitted to the DA layer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SequencerBlockData {
    pub(crate) block_hash: Hash,
    pub(crate) header: Header,
    /// This field should be set for every block with height > 1.
    pub(crate) last_commit: Option<Commit>,
    /// namespace -> rollup data (chain ID and transactions)
    pub(crate) rollup_data: HashMap<Namespace, RollupData>,
    /// The root of the action tree for this block.
    pub(crate) action_tree_root: Hash,
    /// The inclusion proof that the action tree root in included
    /// in `Header::data_hash`.
    pub(crate) action_tree_root_inclusion_proof: InclusionProof,
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
        rollup_data: HashMap<Namespace, RollupData>,
        action_tree_root: Hash,
        action_tree_root_inclusion_proof: InclusionProof,
    ) -> eyre::Result<Self> {
        // perform data validations to ensure only valid [`SequencerBlockData`]
        // can be constructed
        let Some(data_hash) = header.data_hash else {
            bail!(Error::MissingDataHash);
        };
        action_tree_root_inclusion_proof
            .verify(data_hash)
            .wrap_err("failed to verify action tree root inclusion proof")?;
        // TODO: also verify last_commit_hash (#270)

        let data = Self {
            block_hash,
            header,
            last_commit,
            rollup_data,
            action_tree_root,
            action_tree_root_inclusion_proof,
        };

        data.verify_block_hash()
            .wrap_err("block header fields do not merkleize to block hash")?;

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
    pub fn rollup_data(&self) -> &HashMap<Namespace, RollupData> {
        &self.rollup_data
    }

    #[allow(clippy::type_complexity)]
    #[must_use]
    pub fn into_values(
        self,
    ) -> (
        Hash,
        Header,
        Option<Commit>,
        HashMap<Namespace, RollupData>,
        Hash,
        InclusionProof,
    ) {
        (
            self.block_hash,
            self.header,
            self.last_commit,
            self.rollup_data,
            self.action_tree_root,
            self.action_tree_root_inclusion_proof,
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

        let Some(data_hash) = b.header.data_hash else {
            bail!(Error::MissingDataHash);
        };

        if b.data.is_empty() {
            bail!("block has no transactions; ie action tree root is missing");
        }

        let action_tree_root =
            Hash::try_from(b.data[0].clone()).wrap_err("failed to parse action tree root")?;

        // we unwrap sequencer txs into rollup-specific data here,
        // and namespace them correspondingly
        let mut rollup_data = HashMap::new();

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
                    let rollup_data = rollup_data.entry(namespace).or_insert(RollupData {
                        chain_id: action.chain_id().to_vec(),
                        transactions: vec![],
                    });
                    rollup_data.transactions.push(action.data().to_vec());
                }
            });
        }

        // generate the action tree root inclusion proof
        let tx_tree = MerkleTree::from_leaves(b.data);
        let calculated_data_hash = tx_tree.root();
        ensure!(
            // this should never happen for a correctly-constructed block
            calculated_data_hash == data_hash,
            "action tree root does not match the first transaction in the block",
        );
        let action_tree_root_inclusion_proof = tx_tree
            .prove_inclusion(0) // action tree root is always the first tx in a block
            .expect("failed to generate inclusion proof for action tree root");

        let data = Self {
            block_hash: b.header.hash(),
            header: b.header,
            last_commit: b.last_commit,
            rollup_data,
            action_tree_root,
            action_tree_root_inclusion_proof,
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
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use astria_sequencer_validation::{
        InclusionProof,
        MerkleTree,
    };
    use tendermint::Hash;

    use super::SequencerBlockData;
    use crate::{
        sequencer_block_data::RollupData,
        DEFAULT_NAMESPACE,
    };

    fn test_root_and_inclusion_proof() -> (Hash, InclusionProof) {
        let leaves = vec![
            vec![0x11, 0x22, 0x33],
            vec![0x44, 0x55, 0x66],
            vec![0x77, 0x88, 0x99],
        ];
        let tree = MerkleTree::from_leaves(leaves);
        (tree.root(), tree.prove_inclusion(1).unwrap())
    }

    #[test]
    fn sequencer_block_to_bytes() {
        let header = crate::test_utils::default_header();
        let block_hash = header.hash();
        let (action_tree_root, action_tree_root_inclusion_proof) = test_root_and_inclusion_proof();
        let mut expected = SequencerBlockData {
            block_hash,
            header,
            last_commit: None,
            rollup_data: HashMap::new(),
            action_tree_root,
            action_tree_root_inclusion_proof,
        };
        expected.rollup_data.insert(
            DEFAULT_NAMESPACE,
            RollupData {
                chain_id: vec![1, 2, 3], // arbitrary
                transactions: vec![vec![0x44, 0x55, 0x66]],
            },
        );

        let bytes = expected.to_bytes().unwrap();
        let actual = SequencerBlockData::from_bytes(&bytes).unwrap();
        assert_eq!(expected, actual);
    }
}
