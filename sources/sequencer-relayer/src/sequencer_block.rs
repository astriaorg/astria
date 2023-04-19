use base64::{engine::general_purpose, Engine as _};
use eyre::{bail, ensure, WrapErr as _};
use hex;
use prost::{DecodeError, Message};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::debug;

use crate::base64_string::Base64String;
use crate::proto::SequencerMsg;
use crate::proto::{TxBody, TxRaw};
use crate::transaction::txs_to_data_hash;
use crate::types::{Block, Header};

/// Cosmos SDK message type URL for SequencerMsgs.
static SEQUENCER_TYPE_URL: &str = "/SequencerMsg";

/// The default namespace blocks are written to.
/// A block in this namespace contains "pointers" to the rollup txs contained
/// in that block; ie. a list of tuples of (DA block height, namespace).
pub static DEFAULT_NAMESPACE: Namespace = Namespace(*b"astriasq");

/// Namespace represents a Celestia namespace.
#[derive(Clone, Deserialize, Serialize, Debug, Hash, PartialEq, Eq)]
pub struct Namespace([u8; 8]);

impl std::fmt::Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // FIXME: `hex::encode` does an extra allocation which could be removed
        f.write_str(&hex::encode(self.0))
    }
}

impl Namespace {
    pub fn from_string(s: &str) -> eyre::Result<Self> {
        let bytes = hex::decode(s).wrap_err("failed reading string as hex encoded bytes")?;
        ensure!(bytes.len() == 8, "string must encode exactly 8 bytes",);
        let mut namespace = [0u8; 8];
        namespace.copy_from_slice(&bytes);
        Ok(Namespace(namespace))
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexedTransaction {
    pub index: usize,
    pub transaction: Base64String,
}

/// SequencerBlock represents a sequencer layer block to be submitted to
/// the DA layer.
/// TODO: compression or a better serialization method?
/// TODO: rename this b/c it's kind of confusing, types::Block is a cosmos-sdk/tendermint
/// block which is also a sequencer block in a way.
///
/// NOTE: all transactions in this structure are full transaction bytes as received
/// from tendermint.
#[derive(Serialize, Deserialize, Debug)]
pub struct SequencerBlock {
    pub block_hash: Base64String,
    pub header: Header,
    pub sequencer_txs: Vec<IndexedTransaction>,
    /// namespace -> rollup txs
    pub rollup_txs: HashMap<Namespace, Vec<IndexedTransaction>>,
}

impl SequencerBlock {
    /// from_cosmos_block converts a cosmos-sdk block into a SequencerBlock.
    /// it parses the block for SequencerMsgs and namespaces them accordingly.
    pub fn from_cosmos_block(b: Block) -> eyre::Result<Self> {
        if b.header.data_hash.is_none() {
            bail!("block has no data hash");
        }

        // we unwrap generic txs into rollup-specific txs here,
        // and namespace them correspondingly
        let mut sequencer_txs = vec![];
        let mut rollup_txs = HashMap::new();

        for (index, tx) in b.data.txs.iter().enumerate() {
            debug!(
                "parsing tx: {:?}",
                general_purpose::STANDARD.encode(tx.0.clone())
            );

            let tx_body = parse_cosmos_tx(tx)?;
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
                    index,
                    transaction: tx.clone(),
                });
                continue;
            }

            sequencer_txs.push(IndexedTransaction {
                index,
                transaction: tx.clone(),
            })
        }

        Ok(Self {
            block_hash: Base64String(b.header.hash()?.as_bytes().to_vec()),
            header: b.header,
            sequencer_txs,
            rollup_txs,
        })
    }

    /// verify_data_hash verifies that the merkle root of the tree consisting of all the transactions
    /// in the block matches the block's data hash.
    pub fn verify_data_hash(&self) -> eyre::Result<()> {
        let Some(this_data_hash) = self.header.data_hash.as_ref() else {
            bail!("block has no data hash");
        };

        let mut ordered_txs = vec![];
        ordered_txs.append(&mut self.sequencer_txs.clone());
        self.rollup_txs
            .iter()
            .for_each(|(_, tx)| ordered_txs.append(&mut tx.clone()));

        // TODO: if there are duplicate or missing indices, the hash will obviously be wrong,
        // but we should probably verify that earier to return a better error.
        ordered_txs.sort_by(|a, b| a.index.cmp(&b.index));
        let txs = ordered_txs
            .into_iter()
            .map(|tx| tx.transaction)
            .collect::<Vec<_>>();
        let data_hash = txs_to_data_hash(&txs);

        ensure!(
            data_hash.as_bytes() == this_data_hash.0,
            "data hash stored in block header does not match hash calculated from transactions",
        );

        Ok(())
    }

    /// verify_block_hash verifies that the merkle root of the tree consisting of the block header
    /// matches the block's hash.
    pub fn verify_block_hash(&self) -> eyre::Result<()> {
        let block_hash = self.header.hash()?;
        ensure!(
            block_hash.as_bytes() == self.block_hash.0,
            "block hash calculated from tendermint header does not match block hash stored in sequencer block",
        );
        Ok(())
    }
}

pub fn parse_cosmos_tx(tx: &Base64String) -> eyre::Result<TxBody> {
    let tx_raw = TxRaw::decode(tx.0.as_slice())
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
    use super::{cosmos_tx_body_to_sequencer_msgs, parse_cosmos_tx, SEQUENCER_TYPE_URL};
    use crate::base64_string::Base64String;

    #[test]
    fn test_parse_primary_tx() {
        let primary_tx = "CosBCogBChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEmgKLG1ldHJvMXFwNHo0amMwdndxd3hzMnl0NmNrNDRhZWo5bWV5ZnQ0eHg4bXN5EixtZXRybzEwN2Nod2U2MGd2Z3JneXlmbjAybWRsNmxuNjd0dndtOGhyZjR2MxoKCgV1dGljaxIBMRJsClAKRgofL2Nvc21vcy5jcnlwdG8uc2VjcDI1NmsxLlB1YktleRIjCiEDkoWc0MT/06rTUjNPZcvNLqcQJtOvzIWtenGsJXEfEJkSBAoCCAEYBRIYChAKBXV0aWNrEgcxMDAwMDAwEICU69wDGkBeBi44QbvLMvzndkNj+6dckqOR19eNTKV9qZyvtVOrj1+UN/VqeN9Rf0+M6Rmg24uNE5A4jsRcTXh7RkUm9ItT".to_string();
        let tx = parse_cosmos_tx(&Base64String::from_string(primary_tx).unwrap()).unwrap();
        assert_eq!(tx.messages.len(), 1);
        assert_eq!(tx.messages[0].type_url, "/cosmos.bank.v1beta1.MsgSend");
        let sequencer_msgs = cosmos_tx_body_to_sequencer_msgs(tx).unwrap();
        assert_eq!(sequencer_msgs.len(), 0);
    }

    #[test]
    fn test_parse_secondary_tx() {
        let secondary_tx = "Ck0KSwoNL1NlcXVlbmNlck1zZxI6CgNhYWESBWhlbGxvGixtZXRybzFwbHprNzZuamVzdmR0ZnhubTI2dHl5NmV2NGxjYTh3dmZ1M2Q1cxJxClAKRgofL2Nvc21vcy5jcnlwdG8uc2VjcDI1NmsxLlB1YktleRIjCiECjL7oF1zd07+3mCVNz4YHGRleoPDWP08/rGDh14xTkvgSBAoCCAEYBBIYChAKBXV0aWNrEgcxMDAwMDAwEICU69wDIgNhYWEaQMzTIFlWe+yur00V3pXJEZ8uo6AzZ81Q1JJjD+u5EgGDKBslbiabXjPwiRcRMyuHRekBVOGLjNoAPsbhr0F+lTI=".to_string();
        let tx = parse_cosmos_tx(&Base64String::from_string(secondary_tx).unwrap()).unwrap();
        assert_eq!(tx.messages.len(), 1);
        assert_eq!(tx.messages[0].type_url, SEQUENCER_TYPE_URL);
        let sequencer_msgs = cosmos_tx_body_to_sequencer_msgs(tx).unwrap();
        assert_eq!(sequencer_msgs.len(), 1);
        assert_eq!(sequencer_msgs[0].chain_id, "aaa".as_bytes());
        assert_eq!(sequencer_msgs[0].data, "hello".as_bytes());
    }
}
