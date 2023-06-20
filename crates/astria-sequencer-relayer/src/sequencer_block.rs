use std::{
    collections::HashMap,
    fmt,
};

use astria_proto::sequencer::v1::{
    IndexedTransaction as RawIndexedTransaction,
    NamespacedIndexedTransactions,
    SequencerBlock as RawSequencerBlock,
    SequencerMsg,
    TxBody,
    TxRaw,
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
use prost::{
    DecodeError,
    Message as _,
};
use serde::{
    de::{
        self,
        Visitor,
    },
    Deserialize,
    Deserializer,
    Serialize,
};
use sha2::{
    Digest,
    Sha256,
};
use tendermint::{
    hash,
    Hash,
};
use tendermint_proto::Protobuf;
use tracing::debug;

use crate::{
    base64_string::Base64String,
    transaction::txs_to_data_hash,
    types::{
        Block,
        Commit,
        Header,
    },
};

/// Cosmos SDK message type URL for SequencerMsgs.
static SEQUENCER_TYPE_URL: &str = "/SequencerMsg";

/// The default namespace blocks are written to.
/// A block in this namespace contains "pointers" to the rollup txs contained
/// in that block; ie. a list of tuples of (DA block height, namespace).
pub static DEFAULT_NAMESPACE: Namespace = Namespace(*b"astriasq");

/// Namespace represents a Celestia namespace.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Namespace([u8; 8]);

impl Namespace {
    pub fn from_string(s: &str) -> eyre::Result<Self> {
        let bytes =
            hex::decode(s.as_bytes()).wrap_err("failed reading string as hex encoded bytes")?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        ensure!(bytes.len() == 8, "string must encode exactly 8 bytes",);
        let mut namespace = [0u8; 8];
        namespace.copy_from_slice(bytes);
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

impl IndexedTransaction {
    pub fn from_proto(proto: RawIndexedTransaction) -> eyre::Result<Self> {
        Ok(Self {
            block_index: proto.block_index.try_into()?,
            transaction: proto.transaction,
        })
    }

    pub fn to_proto(&self) -> eyre::Result<RawIndexedTransaction> {
        Ok(RawIndexedTransaction {
            block_index: self.block_index as u64,
            transaction: self.transaction.clone(),
        })
    }
}

/// SequencerBlock represents a sequencer layer block to be submitted to
/// the DA layer.
/// TODO: compression or a better serialization method?
/// TODO: rename this b/c it's kind of confusing, types::Block is a cosmos-sdk/tendermint
/// block which is also a sequencer block in a way.
///
/// NOTE: all transactions in this structure are full transaction bytes as received
/// from tendermint.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SequencerBlock {
    pub block_hash: Hash,
    pub header: Header,
    pub last_commit: Commit,
    pub sequencer_transactions: Vec<IndexedTransaction>,
    /// namespace -> rollup txs
    pub rollup_transactions: HashMap<Namespace, Vec<IndexedTransaction>>,
}

impl SequencerBlock {
    pub fn from_proto(proto: RawSequencerBlock) -> eyre::Result<Self> {
        let block_hash = Hash::decode_vec(&proto.block_hash)?;
        let header = Header::from_proto(
            proto
                .header
                .ok_or(eyre!("SequencerBlock from_proto failed: no header"))?,
        )?; // TODO: static errors
        let last_commit = Commit::from_proto(
            proto
                .last_commit
                .ok_or(eyre!("SequencerBlock from_proto failed: no last_commit"))?,
        )?; // TODO: static errors
        let sequencer_txs = proto
            .sequencer_transactions
            .into_iter()
            .map(IndexedTransaction::from_proto)
            .collect::<eyre::Result<Vec<_>>>()?;
        let rollup_txs = proto
            .rollup_transactions
            .into_iter()
            .map(Self::namespaced_indexed_tx_from_proto)
            .collect::<Result<HashMap<Namespace, Vec<IndexedTransaction>>, _>>()?;

        Ok(Self {
            block_hash,
            header,
            last_commit,
            sequencer_transactions: sequencer_txs,
            rollup_transactions: rollup_txs,
        })
    }

    fn namespaced_indexed_tx_from_proto(
        proto: NamespacedIndexedTransactions,
    ) -> eyre::Result<(Namespace, Vec<IndexedTransaction>)> {
        Ok((
            Namespace::from_bytes(&proto.namespace)?,
            proto
                .txs
                .into_iter()
                .map(IndexedTransaction::from_proto)
                .collect::<Result<Vec<IndexedTransaction>, _>>()?,
        ))
    }

    pub fn to_proto(&self) -> eyre::Result<RawSequencerBlock> {
        let block_hash = self.block_hash.encode_vec()?;
        let header = Some(Header::to_proto(&self.header)?);
        let last_commit = Some(Commit::to_proto(&self.last_commit));
        let sequencer_transactions = self
            .sequencer_transactions
            .iter()
            .map(IndexedTransaction::to_proto)
            .collect::<Result<Vec<RawIndexedTransaction>, _>>()?;
        let rollup_transactions = self
            .rollup_transactions
            .iter()
            .map(Self::namespaced_indexed_txs_to_proto)
            .collect::<Result<Vec<NamespacedIndexedTransactions>, _>>()?;

        Ok(RawSequencerBlock {
            block_hash,
            header,
            last_commit,
            sequencer_transactions,
            rollup_transactions,
        })
    }

    fn namespaced_indexed_txs_to_proto(
        (namespace, txs): (&Namespace, &Vec<IndexedTransaction>),
    ) -> eyre::Result<NamespacedIndexedTransactions> {
        Ok(NamespacedIndexedTransactions {
            namespace: namespace.0.to_vec(),
            txs: txs
                .iter()
                .map(IndexedTransaction::to_proto)
                .collect::<Result<Vec<RawIndexedTransaction>, _>>()?,
        })
    }

    pub fn to_bytes(&self) -> eyre::Result<Vec<u8>> {
        Ok(RawSequencerBlock::encode_to_vec(&self.to_proto()?))
    }

    pub fn from_bytes(bytes: &[u8]) -> eyre::Result<Self> {
        Self::from_proto(RawSequencerBlock::decode(bytes)?)
    }

    /// from_cosmos_block converts a cosmos-sdk block into a SequencerBlock.
    /// it parses the block for SequencerMsgs and namespaces them accordingly.
    pub fn from_cosmos_block(b: Block) -> eyre::Result<Self> {
        // we unwrap generic txs into rollup-specific txs here,
        // and namespace them correspondingly
        let mut sequencer_txs = vec![];
        let mut rollup_txs = HashMap::new();

        for (index, tx) in b.data.txs.into_iter().enumerate() {
            debug!("parsing tx: {:?}", general_purpose::STANDARD.encode(&tx.0));

            let tx_body = parse_cosmos_tx(&tx.0)?;
            let msgs = cosmos_tx_body_to_sequencer_msgs(tx_body)?;

            // NOTE: we currently write the entire cosmos tx to Celestia.
            // we kind of have to do this, even though the content of a SequencerMsg is
            // what's relevant, because we need the full tx to reconstruct the data_hash
            // for verification.
            // the logic here is a bit weird; if the tx only contains one message that's
            // destined for a rollup, it's written to the rollup namespace, otherwise
            // it's written to the base namespace.
            if msgs.len() == 1 {
                // TODO: should we allow empty chain IDs?
                let namespace = get_namespace(&msgs[0].chain_id);
                let txs = rollup_txs.entry(namespace).or_insert(vec![]);
                txs.push(IndexedTransaction {
                    block_index: index,
                    transaction: tx.0.clone(),
                });
                continue;
            }

            sequencer_txs.push(IndexedTransaction {
                block_index: index,
                transaction: tx.0.clone(),
            })
        }

        Ok(Self {
            block_hash: b.header.hash()?,
            header: b.header,
            last_commit: b.last_commit,
            sequencer_transactions: sequencer_txs,
            rollup_transactions: rollup_txs,
        })
    }

    /// verify_data_hash verifies that the merkle root of the tree consisting of all the
    /// transactions in the block matches the block's data hash.
    pub fn verify_data_hash(&self) -> eyre::Result<()> {
        let Some(this_data_hash) = self.header.data_hash.clone() else {
            bail!("block has no data hash");
        };

        let mut ordered_txs = vec![];
        ordered_txs.append(&mut self.sequencer_transactions.clone());
        self.rollup_transactions
            .iter()
            .for_each(|(_, tx)| ordered_txs.append(&mut tx.clone()));

        // TODO: if there are duplicate or missing indices, the hash will obviously be wrong,
        // but we should probably verify that earier to return a better error.
        ordered_txs.sort_by(|a, b| a.block_index.cmp(&b.block_index));
        let txs = ordered_txs
            .into_iter()
            .map(|tx| Base64String::from_bytes(&tx.transaction))
            .collect::<Vec<_>>();
        let data_hash = txs_to_data_hash(&txs);

        ensure!(
            data_hash == Hash::from_bytes(hash::Algorithm::Sha256, &this_data_hash.0)?,
            "data hash stored in block header does not match hash calculated from transactions",
        );

        Ok(())
    }

    /// verify_block_hash verifies that the merkle root of the tree consisting of the block header
    /// matches the block's hash.
    pub fn verify_block_hash(&self) -> eyre::Result<()> {
        let block_hash = self.header.hash()?;
        ensure!(
            block_hash == self.block_hash,
            "block hash calculated from tendermint header does not match block hash stored in \
             sequencer block",
        );
        Ok(())
    }
}

pub fn parse_cosmos_tx(tx: &Vec<u8>) -> eyre::Result<TxBody> {
    let tx_raw = TxRaw::decode(tx.as_slice())
        .wrap_err("failed decoding raw tx protobuf from hex encoded transaction")?;
    let tx_body = TxBody::decode(tx_raw.body_bytes.as_slice())
        .wrap_err("failed decoding tx body from protobuf stored in raw tx body bytes")?;
    Ok(tx_body)
}

pub fn cosmos_tx_body_to_sequencer_msgs(tx_body: TxBody) -> eyre::Result<Vec<SequencerMsg>> {
    tx_body
        .messages
        .iter()
        .filter(|msg| msg.type_url == SEQUENCER_TYPE_URL)
        .map(|msg| SequencerMsg::decode(msg.value.as_slice()))
        .collect::<Result<Vec<SequencerMsg>, DecodeError>>()
        .wrap_err("failed decoding sequencer msg from value stored in cosmos tx body")
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use tendermint::{
        block::Height,
        hash,
        Hash,
        Time,
    };

    use super::{
        cosmos_tx_body_to_sequencer_msgs,
        parse_cosmos_tx,
        Header,
        SequencerBlock,
        DEFAULT_NAMESPACE,
        SEQUENCER_TYPE_URL,
    };
    use crate::{
        base64_string::Base64String,
        sequencer_block::IndexedTransaction,
        types::{
            BlockId,
            Commit,
            Parts,
            Version,
        },
    };

    fn make_header() -> Header {
        Header {
            version: Version {
                block: 0,
                app: 0,
            },
            chain_id: String::from("chain"),
            height: Height::from(0_u32),
            time: Time::now().to_string(),
            last_block_id: Some(make_block_id()),
            last_commit_hash: Some(Base64String::from_bytes(&[0; 32])),
            data_hash: Some(Base64String::from_bytes(&[0; 32])),
            validators_hash: Base64String::from_bytes(&[0; 32]),
            next_validators_hash: Base64String::from_bytes(&[0; 32]),
            consensus_hash: Base64String::from_bytes(&[0; 32]),
            app_hash: Base64String::from_bytes(&[0; 32]),
            last_results_hash: Some(Base64String::from_bytes(&[0; 32])),
            evidence_hash: Some(Base64String::from_bytes(&[0; 32])),
            proposer_address: Base64String::from_bytes(&[0; 20]),
        }
    }

    fn empty_commit() -> Commit {
        Commit {
            height: Height::from(0u32),
            round: 0,
            block_id: BlockId {
                hash: Base64String::from_bytes(&[0; 32]),
                part_set_header: Parts {
                    total: 0,
                    hash: Base64String::from_bytes(&[0; 32]),
                },
            },
            signatures: vec![],
        }
    }

    fn make_block_id() -> BlockId {
        BlockId {
            hash: Base64String::from_bytes(&[0; 32]),
            part_set_header: Parts {
                total: 0,
                hash: Base64String::from_bytes(&[0; 32]),
            },
        }
    }

    #[test]
    fn test_parse_primary_tx() {
        let primary_tx = "CosBCogBChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEmgKLG1ldHJvMXFwNHo0amMwdndxd3hzMnl0NmNrNDRhZWo5bWV5ZnQ0eHg4bXN5EixtZXRybzEwN2Nod2U2MGd2Z3JneXlmbjAybWRsNmxuNjd0dndtOGhyZjR2MxoKCgV1dGljaxIBMRJsClAKRgofL2Nvc21vcy5jcnlwdG8uc2VjcDI1NmsxLlB1YktleRIjCiEDkoWc0MT/06rTUjNPZcvNLqcQJtOvzIWtenGsJXEfEJkSBAoCCAEYBRIYChAKBXV0aWNrEgcxMDAwMDAwEICU69wDGkBeBi44QbvLMvzndkNj+6dckqOR19eNTKV9qZyvtVOrj1+UN/VqeN9Rf0+M6Rmg24uNE5A4jsRcTXh7RkUm9ItT".to_string();
        let tx = parse_cosmos_tx(&Base64String::from_string(primary_tx).unwrap().0).unwrap();
        assert_eq!(tx.messages.len(), 1);
        assert_eq!(tx.messages[0].type_url, "/cosmos.bank.v1beta1.MsgSend");
        let sequencer_msgs = cosmos_tx_body_to_sequencer_msgs(tx).unwrap();
        assert_eq!(sequencer_msgs.len(), 0);
    }

    #[test]
    fn test_parse_secondary_tx() {
        let secondary_tx = "Ck0KSwoNL1NlcXVlbmNlck1zZxI6CgNhYWESBWhlbGxvGixtZXRybzFwbHprNzZuamVzdmR0ZnhubTI2dHl5NmV2NGxjYTh3dmZ1M2Q1cxJxClAKRgofL2Nvc21vcy5jcnlwdG8uc2VjcDI1NmsxLlB1YktleRIjCiECjL7oF1zd07+3mCVNz4YHGRleoPDWP08/rGDh14xTkvgSBAoCCAEYBBIYChAKBXV0aWNrEgcxMDAwMDAwEICU69wDIgNhYWEaQMzTIFlWe+yur00V3pXJEZ8uo6AzZ81Q1JJjD+u5EgGDKBslbiabXjPwiRcRMyuHRekBVOGLjNoAPsbhr0F+lTI=".to_string();
        let tx = parse_cosmos_tx(&Base64String::from_string(secondary_tx).unwrap().0).unwrap();
        assert_eq!(tx.messages.len(), 1);
        assert_eq!(tx.messages[0].type_url, SEQUENCER_TYPE_URL);
        let sequencer_msgs = cosmos_tx_body_to_sequencer_msgs(tx).unwrap();
        assert_eq!(sequencer_msgs.len(), 1);
        assert_eq!(sequencer_msgs[0].chain_id, "aaa".as_bytes());
        assert_eq!(sequencer_msgs[0].data, "hello".as_bytes());
    }

    #[test]
    fn decode_sequencer_block_json() {
        let block_string = r#"{
            "block_id": {
              "hash": "X7FkXtx8dCCB67ajTNB9V5+SmFbDS5h1ToXh/t0ETWE=",
              "part_set_header": {
                "total": 1,
                "hash": "UyqowpfIUW05Ca+u83WJaKTO/QcXqU8qXrUpU87DT98="
              }
            },
            "block": {
              "header": {
                "version": {
                  "block": 11,
                  "app": 0
                },
                "chain_id": "private",
                "height": "2701",
                "time": "2023-06-16T18:15:52.426568223Z",
                "last_block_id": {
                  "hash": "wgwPZZnVkXmUGV7RvEpBoorr+SFOpO/4JEXXnQU+eag=",
                  "part_set_header": {
                    "total": 1,
                    "hash": "Xyye/Sn/F2ZP2WwzGSy5BLPeDJoH4GaO4mj2xNi6N+M="
                  }
                },
                "last_commit_hash": "Lvf4S09arOG46br0xIvJO+VfisZ/5+MXc/o5a3jJlCI=",
                "data_hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
                "validators_hash": "byEpnFOJ6Nlinv4mNALU1wuDXPH3DgXG78G3Q+391ro=",
                "next_validators_hash": "byEpnFOJ6Nlinv4mNALU1wuDXPH3DgXG78G3Q+391ro=",
                "consensus_hash": "BICRvH3cKD93v7+R1zxE2ljD34qcvIZ0Bdi389qtoi8=",
                "app_hash": "BFX6odsVLKLHUTFlJ7iHmvWqD1efg8d1jXi7bNviBfI=",
                "last_results_hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
                "evidence_hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
                "proposer_address": "ZqWPb3ULjzbtgFmb7t+tJeEegGk="
              },
              "data": {
                "txs": [
                ],
                "blobs": [
                ],
                "square_size": "0",
                "hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="
              },
              "evidence": {
                "evidence": [
                ]
              },
              "last_commit": {
                "height": "2700",
                "round": 0,
                "block_id": {
                  "hash": "wgwPZZnVkXmUGV7RvEpBoorr+SFOpO/4JEXXnQU+eag=",
                  "part_set_header": {
                    "total": 1,
                    "hash": "Xyye/Sn/F2ZP2WwzGSy5BLPeDJoH4GaO4mj2xNi6N+M="
                  }
                },
                "signatures": [
                  {
                    "block_id_flag": "BLOCK_ID_FLAG_COMMIT",
                    "validator_address": "ZqWPb3ULjzbtgFmb7t+tJeEegGk=",
                    "timestamp": "2023-06-16T18:15:52.426568223Z",
                    "signature": "MC4H9dPFmyzxhJdRhplJngX05O9/9t2hm39lIJQZ8AOpgih4IJPJq18abmyQCkQNMEvaJYFoh+fLsxf6MtSgAA=="
                  }
                ]
              }
            }
          }"#;
        serde_json::from_str::<crate::types::BlockResponse>(block_string).unwrap();
    }

    #[test]
    fn sequencer_block_to_bytes_round_trip() {
        let mut expected = SequencerBlock {
            block_hash: Hash::from_bytes(
                hash::Algorithm::Sha256,
                &Base64String::from_string(
                    "Ojskac/Fi5G00alQZms+tdtIox53cWWjBmIGEnWG1+M=".to_string(),
                )
                .unwrap()
                .0,
            )
            .unwrap(),
            header: make_header(),
            last_commit: empty_commit(),
            sequencer_transactions: vec![IndexedTransaction {
                block_index: 0,
                transaction: vec![0x11, 0x22, 0x33],
            }],
            rollup_transactions: HashMap::new(),
        };
        expected.rollup_transactions.insert(
            DEFAULT_NAMESPACE.clone(),
            vec![IndexedTransaction {
                block_index: 0,
                transaction: vec![0x44, 0x55, 0x66],
            }],
        );

        let bytes = expected.to_bytes().unwrap();
        let actual = SequencerBlock::from_bytes(&bytes).unwrap();
        assert_eq!(expected, actual);
    }
}
