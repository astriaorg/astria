use std::{
    collections::HashMap,
    fmt,
};

use astria_sequencer_client::{
    Action,
    SignedTransaction,
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
use hex;
use serde::{
    de::{
        self,
        Visitor,
    },
    Deserialize,
    Deserializer,
    Serialize,
};
use serde_json;
use sha2::{
    Digest,
    Sha256,
};
use tendermint::{
    block::{
        Commit,
        Header,
    },
    Block,
};
use tracing::debug;

use crate::base64_string::Base64String;

/// The default namespace blocks are written to.
/// A block in this namespace contains "pointers" to the rollup txs contained
/// in that block; ie. a list of tuples of (DA block height, namespace).
pub static DEFAULT_NAMESPACE: Namespace = Namespace(*b"astriasq");

/// Namespace represents a Celestia namespace.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Namespace([u8; 8]);

impl Namespace {
    pub fn from_string(s: &str) -> eyre::Result<Self> {
        let bytes = hex::decode(s).wrap_err("failed reading string as hex encoded bytes")?;
        ensure!(bytes.len() == 8, "string must encode exactly 8 bytes",);
        let mut namespace = [0u8; 8];
        namespace.copy_from_slice(&bytes);
        Ok(Namespace(namespace))
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // FIXME: `hex::encode` does an extra allocation which could be removed
        f.write_str(&hex::encode(self.0))
    }
}

impl Serialize for Namespace {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for Namespace {
    fn deserialize<D>(deserializer: D) -> Result<Namespace, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(NamespaceVisitor)
    }
}

struct NamespaceVisitor;

impl NamespaceVisitor {
    fn decode_string<E>(self, value: &str) -> Result<Namespace, E>
    where
        E: de::Error,
    {
        Namespace::from_string(value).map_err(|e| de::Error::custom(format!("{e:?}")))
    }
}

impl<'de> Visitor<'de> for NamespaceVisitor {
    type Value = Namespace;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string containing 8 hex-encoded bytes")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.decode_string(value)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.decode_string(&value)
    }
}

/// get_namespace returns an 8-byte namespace given a byte slice.
pub fn get_namespace(bytes: &[u8]) -> Namespace {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    Namespace(result[0..8].to_owned().try_into().unwrap())
}

/// IndexedTransaction represents a sequencer transaction along with the index
/// it was originally in in the sequencer block.
/// This is required so that the block's `data_hash`, which is a merkle root
/// of the transactions in the block, can be verified.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct IndexedTransaction {
    pub block_index: usize,
    pub transaction: Vec<u8>,
}

/// ParsedSequencerBlockData represents a sequencer block's data
/// to be submitted to the DA layer.
///
/// TODO: compression or a better serialization method?
/// TODO: merkle proofs for each rollup's transactions
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct ParsedSequencerBlockData {
    pub block_hash: Base64String,
    pub header: Header,
    /// This field should be set for every block with height > 1.
    pub last_commit: Option<Commit>,
    /// namespace -> rollup txs
    pub rollup_txs: HashMap<Namespace, Vec<IndexedTransaction>>,
}

impl ParsedSequencerBlockData {
    pub fn to_bytes(&self) -> eyre::Result<Vec<u8>> {
        // TODO: don't use json, use our own serializer (or protobuf for now?)
        serde_json::to_vec(self).wrap_err("failed serializing signed namespace data to json")
    }

    pub fn from_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        serde_json::from_slice(bytes)
            .wrap_err("failed deserializing signed namespace data from bytes")
    }

    /// Converts a Tendermint block into a `ParsedSequencerBlockData`.
    /// it parses the block for `SequenceAction`s and namespaces them accordingly.
    pub fn from_tendermint_block(b: Block) -> eyre::Result<Self> {
        if b.header.data_hash.is_none() {
            bail!("block has no data hash");
        }

        // we unwrap sequencer txs into rollup-specific data here,
        // and namespace them correspondingly
        let mut rollup_txs = HashMap::new();

        for (index, tx) in b.data.iter().enumerate() {
            debug!(
                "parsing tx: {:?}",
                general_purpose::STANDARD.encode(tx.as_slice())
            );

            let tx = parse_sequencer_tx(tx)?;
            // let msgs = cosmos_tx_body_to_sequencer_msgs(tx_body)?;

            for action in tx.transaction.actions {
                if let Action::SequenceAction(action) = action {
                    let namespace = get_namespace(&action.chain_id);
                    let txs = rollup_txs.entry(namespace).or_insert(vec![]);
                    txs.push(IndexedTransaction {
                        block_index: index,
                        transaction: action.data.clone(),
                    });
                }
            }
        }

        Ok(Self {
            block_hash: Base64String(b.header.hash().as_bytes().to_vec()),
            header: b.header,
            last_commit: b.last_commit,
            rollup_txs,
        })
    }

    /// verify_data_hash verifies that the merkle root of the tree consisting of all the
    /// transactions in the block matches the block's data hash.
    ///
    /// TODO: this breaks with the update to use Retro; need to update for merkle proofs
    pub fn verify_data_hash(&self) -> eyre::Result<()> {
        Ok(())
    }

    /// verify_block_hash verifies that the merkle root of the tree consisting of the block header
    /// matches the block's hash.
    pub fn verify_block_hash(&self) -> eyre::Result<()> {
        let block_hash = self.header.hash();
        ensure!(
            block_hash.as_bytes() == self.block_hash.0,
            "block hash calculated from tendermint header does not match block hash stored in \
             sequencer block",
        );
        Ok(())
    }
}

fn parse_sequencer_tx(tx: &[u8]) -> eyre::Result<SignedTransaction> {
    SignedTransaction::try_from_slice(tx).map_err(|e| eyre!(e))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::{
        IndexedTransaction,
        ParsedSequencerBlockData,
        DEFAULT_NAMESPACE,
    };
    use crate::base64_string::Base64String;

    #[test]
    fn sequencer_block_to_bytes() {
        let mut expected = ParsedSequencerBlockData {
            block_hash: Base64String::from_string(
                "Ojskac/Fi5G00alQZms+tdtIox53cWWjBmIGEnWG1+M=".to_string(),
            )
            .unwrap(),
            header: crate::utils::default_header(),
            last_commit: None,
            rollup_txs: HashMap::new(),
        };
        expected.rollup_txs.insert(
            DEFAULT_NAMESPACE.clone(),
            vec![IndexedTransaction {
                block_index: 0,
                transaction: vec![0x44, 0x55, 0x66],
            }],
        );

        let bytes = expected.to_bytes().unwrap();
        let actual = ParsedSequencerBlockData::from_bytes(&bytes).unwrap();
        assert_eq!(expected, actual);
    }
}
