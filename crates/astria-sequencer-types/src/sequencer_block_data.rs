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
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct RollupData {
    #[serde(with = "hex::serde")]
    pub chain_id: Vec<u8>,
    pub transactions: Vec<Vec<u8>>,
}

/// `SequencerBlockData` represents a sequencer block's data
/// to be submitted to the DA layer.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(try_from = "RawSequencerBlockData")]
#[serde(into = "RawSequencerBlockData")]
pub struct SequencerBlockData {
    block_hash: Hash,
    header: Header,
    /// This field should be set for every block with height > 1.
    last_commit: Option<Commit>,
    /// namespace -> rollup data (chain ID and transactions)
    rollup_data: HashMap<Namespace, RollupData>,
    /// The root of the action tree for this block.
    action_tree_root: [u8; 32],
    /// The inclusion proof that the action tree root is included
    /// in `Header::data_hash`.
    action_tree_root_inclusion_proof: InclusionProof,
}

impl SequencerBlockData {
    /// Creates a new `SequencerBlockData` from the given data.
    ///
    /// # Errors
    ///
    /// - if the block hash does not correspond to the hashed header provided
    pub fn try_from_raw(raw: RawSequencerBlockData) -> eyre::Result<Self> {
        let RawSequencerBlockData {
            block_hash,
            header,
            last_commit,
            rollup_data,
            action_tree_root,
            action_tree_root_inclusion_proof,
        } = raw;

        let Some(data_hash) = header.data_hash else {
            bail!(Error::MissingDataHash);
        };
        action_tree_root_inclusion_proof
            .verify(&action_tree_root, data_hash)
            .wrap_err("failed to verify action tree root inclusion proof")?;
        // TODO(https://github.com/astriaorg/astria/issues/270): also verify last_commit_hash

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

    /// Returns the hash in the last block id, unless this block is genesis.
    pub fn parent_block_hash(&self) -> Option<Hash> {
        self.header.last_block_id.map(|id| id.hash)
    }

    /// Returns the [`SequencerBlockData`] as a [`RawSequencerBlockData`].
    #[must_use]
    pub fn into_raw(self) -> RawSequencerBlockData {
        let Self {
            block_hash,
            header,
            last_commit,
            rollup_data,
            action_tree_root,
            action_tree_root_inclusion_proof,
        } = self;

        RawSequencerBlockData {
            block_hash,
            header,
            last_commit,
            rollup_data,
            action_tree_root,
            action_tree_root_inclusion_proof,
        }
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
    /// - if the block has no transactions
    /// - if the block's first transaction is not the 32-byte action tree root
    /// - if a transaction in the block cannot be parsed
    /// - if the block's `data_hash` does not match the one calculated from the transactions
    /// - if the inclusion proof of the action tree root in the block's `data_hash` cannot be
    ///   generated
    ///
    /// See `specs/sequencer-inclusion-proofs.md` for most details on the action tree root
    /// and inclusion proof purpose.
    pub fn from_tendermint_block(b: Block) -> eyre::Result<Self> {
        use proto::{
            generated::sequencer::v1alpha1 as raw,
            native::sequencer::v1alpha1::SignedTransaction,
            Message as _,
        };
        let Some(data_hash) = b.header.data_hash else {
            bail!(Error::MissingDataHash);
        };

        if b.data.is_empty() {
            bail!("block has no transactions; ie action tree root is missing");
        }

        let action_tree_root: [u8; 32] = b.data[0]
            .clone()
            .try_into()
            .map_err(|_| eyre!("action tree root must be 32 bytes"))?;

        // we unwrap sequencer txs into rollup-specific data here,
        // and namespace them correspondingly
        let mut rollup_data = HashMap::new();

        // the first transaction is skipped as it's the action tree root,
        // not a user-submitted transaction.
        for (index, tx) in b.data[1..].iter().enumerate() {
            debug!(
                index,
                bytes = general_purpose::STANDARD.encode(tx.as_slice()),
                "parsing data from tendermint block",
            );

            let raw_tx = raw::SignedTransaction::decode(&**tx)
                .wrap_err("failed decoding bytes to protobuf signed transaction")?;
            let tx = SignedTransaction::try_from_raw(raw_tx).wrap_err(
                "failed constructing native signed transaction from raw protobuf signed \
                 transaction",
            )?;
            tx.actions().iter().for_each(|action| {
                if let Some(action) = action.as_sequence() {
                    // TODO(https://github.com/astriaorg/astria/issues/318): intern
                    // these namespaces so they don't get rebuild on every iteration.
                    let namespace = Namespace::from_slice(&action.chain_id);
                    rollup_data
                        .entry(namespace)
                        .and_modify(|data: &mut RollupData| {
                            data.transactions.push(action.data.clone());
                        })
                        .or_insert_with(|| RollupData {
                            chain_id: action.chain_id.clone(),
                            transactions: vec![action.data.clone()],
                        });
                }
            });
        }

        // generate the action tree root proof of inclusion in `Header::data_hash`
        let tx_tree = MerkleTree::from_leaves(b.data);
        let calculated_data_hash = tx_tree.root();
        ensure!(
            // this should never happen for a correctly-constructed block
            calculated_data_hash == data_hash.as_bytes(),
            "action tree root does not match the first transaction in the block",
        );
        let action_tree_root_inclusion_proof = tx_tree
            .prove_inclusion(0) // action tree root is always the first tx in a block
            .wrap_err("failed to generate inclusion proof for action tree root")?;

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

/// An unverified version of [`SequencerBlockData`], primarily used for
/// serialization/deserialization.
#[allow(clippy::module_name_repetitions)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RawSequencerBlockData {
    pub block_hash: Hash,
    pub header: Header,
    /// This field should be set for every block with height > 1.
    pub last_commit: Option<Commit>,
    /// namespace -> rollup data (chain ID and transactions)
    pub rollup_data: HashMap<Namespace, RollupData>,
    /// The root of the action tree for this block.
    pub action_tree_root: [u8; 32],
    /// The inclusion proof that the action tree root is included
    /// in `Header::data_hash`.
    pub action_tree_root_inclusion_proof: InclusionProof,
}

impl TryFrom<RawSequencerBlockData> for SequencerBlockData {
    type Error = eyre::Error;

    fn try_from(raw: RawSequencerBlockData) -> eyre::Result<Self> {
        Self::try_from_raw(raw)
    }
}

impl From<SequencerBlockData> for RawSequencerBlockData {
    fn from(data: SequencerBlockData) -> Self {
        data.into_raw()
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use astria_sequencer_validation::MerkleTree;
    use tendermint::Hash;

    use super::SequencerBlockData;
    use crate::RawSequencerBlockData;

    #[test]
    fn new_sequencer_block() {
        let mut header = crate::test_utils::default_header();
        let (action_tree_root, action_tree_root_inclusion_proof, data_hash) = {
            let action_tree_root = [9u8; 32];
            let transactions = vec![
                action_tree_root.to_vec(),
                vec![0x11, 0x22, 0x33],
                vec![0x44, 0x55, 0x66],
                vec![0x77, 0x88, 0x99],
            ];
            let tree = MerkleTree::from_leaves(transactions);
            (
                action_tree_root,
                tree.prove_inclusion(0).unwrap(),
                tree.root(),
            )
        };

        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());
        let block_hash = header.hash();
        SequencerBlockData::try_from_raw(RawSequencerBlockData {
            block_hash,
            header,
            last_commit: None,
            rollup_data: HashMap::new(),
            action_tree_root,
            action_tree_root_inclusion_proof,
        })
        .unwrap();
    }

    #[test]
    fn sequencer_block_to_bytes() {
        let mut header = crate::test_utils::default_header();
        let (action_tree_root, action_tree_root_inclusion_proof, data_hash) = {
            let action_tree_root = [9u8; 32];
            let transactions = vec![
                action_tree_root.to_vec(),
                vec![0x11, 0x32, 0x33],
                vec![0x44, 0x55, 0x66],
                vec![0x77, 0x88, 0x99],
            ];
            let tree = MerkleTree::from_leaves(transactions);
            (
                action_tree_root,
                tree.prove_inclusion(0).unwrap(),
                tree.root(),
            )
        };

        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());
        let block_hash = header.hash();
        let data = SequencerBlockData::try_from_raw(RawSequencerBlockData {
            block_hash,
            header,
            last_commit: None,
            rollup_data: HashMap::new(),
            action_tree_root,
            action_tree_root_inclusion_proof,
        })
        .unwrap();

        let bytes = data.to_bytes().unwrap();
        let actual = SequencerBlockData::from_bytes(&bytes).unwrap();
        assert_eq!(data, actual);
    }
}
