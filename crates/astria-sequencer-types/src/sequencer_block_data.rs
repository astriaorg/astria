use std::collections::HashMap;

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
};
use tracing::debug;

use crate::namespace::Namespace;

/// `SequencerBlockData` represents a sequencer block's data
/// to be submitted to the DA layer.
///
/// TODO: compression or a better serialization method?
/// TODO: merkle proofs for each rollup's transactions
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct SequencerBlockData {
    #[serde(with = "crate::serde::Base64Standard")]
    pub block_hash: Vec<u8>,
    pub header: Header,
    /// This field should be set for every block with height > 1.
    pub last_commit: Option<Commit>,
    /// namespace -> rollup txs
    pub rollup_txs: HashMap<Namespace, Vec<Vec<u8>>>,
}

impl SequencerBlockData {
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
            bail!("block has no data hash");
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
                    let namespace = Namespace::new_from_bytes(action.chain_id());
                    let txs = rollup_txs.entry(namespace).or_insert(vec![]);
                    txs.push(action.data().to_vec());
                }
            });
        }

        let data = Self {
            block_hash: b.header.hash().as_bytes().to_vec(),
            header: b.header,
            last_commit: b.last_commit,
            rollup_txs,
        };
        Ok(data)
    }

    /// verifies that the merkle root of the tree consisting of all the
    /// transactions in the block matches the block's data hash.
    ///
    /// TODO: this breaks with the update to use Retro; need to update for merkle proofs
    ///
    /// # Errors
    ///
    /// - unimplemented
    pub fn verify_data_hash(&self) -> eyre::Result<()> {
        Ok(())
    }

    /// verifies that the merkle root of the tree consisting of the block header
    /// matches the block's hash.
    ///
    /// # Errors
    ///
    /// - if the block hash calculated from the header does not match the block hash stored
    ///  in the sequencer block
    pub fn verify_block_hash(&self) -> eyre::Result<()> {
        let block_hash = self.header.hash();
        ensure!(
            block_hash.as_bytes() == self.block_hash,
            "block hash calculated from tendermint header does not match block hash stored in \
             sequencer block",
        );
        Ok(())
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
            block_hash: block_hash.as_bytes().to_vec(),
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
