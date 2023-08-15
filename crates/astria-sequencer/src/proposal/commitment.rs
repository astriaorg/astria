use std::collections::BTreeMap;

use bytes::Bytes;
use tendermint::merkle::simple_hash_from_byte_vectors;

use crate::transaction::Signed;

/// Called when we receive a `PrepareProposal` or `ProcessProposal` consensus message.
///
/// In the case of `PrepareProposal`, we use this function to generate the `commitment_tx`
/// to be placed at the start of the block.
///
/// In the case of `ProcessProposal`, we use this function to generate and verify the
/// `commitment_tx` expected at the start of the block.
///
/// This function sorts the block's `sequence::Action`s contained within the transactions
/// using their `chain_id`. It then returns the merkle root of the tree where each leaf is
/// a commitment of `sequence::Action`s with the same `chain_id`. The leaves are ordered
/// by namespace in ascending order, where `namespace(chain_id) = Sha256(chain_id)[0..10]`.
/// This structure can be referred to as the "action tree".
///
/// The leaf, which contains a commitment to every action with the same `chain_id`, is currently
/// implemented as ( `namespace(chain_id)` || root of merkle tree of the `sequence::Action`s ).
/// This is somewhat arbitrary, but could be useful for proof of an action within the action tree.
pub(crate) fn generate_sequence_actions_commitment(
    txs_bytes: Vec<Bytes>,
) -> ([u8; 32], Vec<Bytes>) {
    // ignore any transactions that are not deserializable
    let txs = txs_bytes
        .into_iter()
        .filter_map(|tx_bytes| match Signed::try_from_slice(&tx_bytes) {
            Ok(tx) => Some((tx, tx_bytes)),
            Err(err) => {
                tracing::debug!("failed to deserialize tx bytes {:?}: {}", tx_bytes, err);
                None
            }
        })
        .collect::<Vec<(Signed, Bytes)>>();
    let (signed_txs, txs_to_include): (Vec<Signed>, Vec<Bytes>) = txs.into_iter().unzip();

    let namespace_to_txs = group_sequence_actions_by_chain_id(&signed_txs);

    // each leaf of the action tree is the root of a merkle tree of the `sequence::Action`s
    // with the same `chain_id`, prepended with the 10-byte `namespace(chain_id)`.
    // the leaves are sorted in ascending order by namespace.
    let leaves = generate_action_tree_leaves(&namespace_to_txs);
    (
        simple_hash_from_byte_vectors::<tendermint::crypto::default::Sha256>(&leaves),
        txs_to_include,
    )
}

#[must_use]
pub fn generate_action_tree_leaves(
    namespace_to_txs: &BTreeMap<[u8; 10], Vec<Vec<u8>>>,
) -> Vec<Vec<u8>> {
    let mut leaves: Vec<Vec<u8>> = vec![];
    for (namespace, txs) in namespace_to_txs {
        let chain_id_root =
            simple_hash_from_byte_vectors::<tendermint::crypto::default::Sha256>(txs);
        let mut leaf = namespace.to_vec();
        leaf.append(&mut chain_id_root.to_vec());
        leaves.push(leaf);
    }
    leaves
}

/// returns an 10-byte namespace given a byte slice.
/// TODO: duplicate in `astria-sequencer-relayer/src/types.rs`
fn get_namespace(bytes: &[u8]) -> [u8; 10] {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    result[0..10]
        .to_owned()
        .try_into()
        .expect("cannot fail as hash is always 32 bytes")
}

/// Groups the `sequence::Action`s within the transactions by their `chain_id`.
/// Other types of actions are ignored.
/// Within an entry, actions are ordered by their transaction index within a block.
fn group_sequence_actions_by_chain_id(txs: &[Signed]) -> BTreeMap<[u8; 10], Vec<Vec<u8>>> {
    let mut rollup_txs_map = BTreeMap::new();

    for tx in txs.iter() {
        tx.transaction().actions().iter().for_each(|action| {
            if let Some(action) = action.as_sequence() {
                let txs_for_rollup: &mut Vec<Vec<u8>> = rollup_txs_map
                    .entry(get_namespace(action.chain_id()))
                    .or_insert(vec![]);
                txs_for_rollup.push(action.data().to_vec());
            }
        });
    }

    rollup_txs_map
}

#[cfg(test)]
mod test {
    use ed25519_consensus::SigningKey;
    use prost::Message as _;
    use rand::rngs::OsRng;

    use super::*;
    use crate::{
        accounts::{
            self,
            types::{
                Address,
                Balance,
                Nonce,
            },
        },
        sequence,
        transaction::{
            action::Action,
            Unsigned,
        },
    };

    #[test]
    fn generate_sequence_actions_commitment_should_ignore_transfers() {
        let sequence_action = Action::SequenceAction(sequence::Action::new(
            b"testchainid".to_vec(),
            b"helloworld".to_vec(),
        ));

        let signing_key = SigningKey::new(OsRng);
        let tx = Unsigned {
            nonce: Nonce::from(0),
            actions: vec![
                sequence_action.clone(),
                Action::TransferAction(accounts::Transfer::new(
                    Address([0u8; 20]),
                    Balance::from(1),
                )),
            ],
        };

        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.to_proto().encode_to_vec();
        let txs = vec![tx_bytes.into()];
        let (action_commitment_0, _) = generate_sequence_actions_commitment(txs);

        let signing_key = SigningKey::new(OsRng);
        let tx = Unsigned {
            nonce: Nonce::from(0),
            actions: vec![sequence_action],
        };

        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.to_proto().encode_to_vec();
        let txs = vec![tx_bytes.into()];
        let (action_commitment_1, _) = generate_sequence_actions_commitment(txs);
        assert_eq!(action_commitment_0, action_commitment_1);
    }

    #[test]
    fn generate_action_tree_leaves_assert_leaves_ordered_by_namespace() {
        let signing_key = SigningKey::new(OsRng);

        let namespace_0 = get_namespace(b"testchainid0");
        let tx = Unsigned {
            nonce: Nonce::from(0),
            actions: vec![Action::SequenceAction(sequence::Action::new(
                namespace_0.to_vec(),
                b"helloworld".to_vec(),
            ))],
        };
        let signed_tx_0 = tx.into_signed(&signing_key);

        let namespace_1 = get_namespace(b"testchainid1");
        let tx = Unsigned {
            nonce: Nonce::from(0),
            actions: vec![Action::SequenceAction(sequence::Action::new(
                namespace_1.to_vec(),
                b"helloworld".to_vec(),
            ))],
        };
        let signed_tx_1 = tx.into_signed(&signing_key);

        let namespace_2 = get_namespace(b"testchainid2");
        let tx = Unsigned {
            nonce: Nonce::from(0),
            actions: vec![Action::SequenceAction(sequence::Action::new(
                namespace_2.to_vec(),
                b"helloworld".to_vec(),
            ))],
        };
        let signed_tx_2 = tx.into_signed(&signing_key);

        let txs = vec![signed_tx_0, signed_tx_1, signed_tx_2];
        let namespace_to_txs = group_sequence_actions_by_chain_id(&txs);
        let leaves = generate_action_tree_leaves(&namespace_to_txs);
        leaves.iter().enumerate().for_each(|(i, leaf)| {
            if i == 0 {
                return;
            }
            assert!(leaf[0..10] > leaves[i - 1][0..10]);
        });
    }

    #[test]
    fn generate_sequence_actions_commitment_snapshot() {
        let sequence_action = Action::SequenceAction(sequence::Action::new(
            b"testchainid".to_vec(),
            b"helloworld".to_vec(),
        ));

        let signing_key = SigningKey::new(OsRng);
        let tx = Unsigned {
            nonce: Nonce::from(0),
            actions: vec![
                sequence_action,
                Action::TransferAction(accounts::Transfer::new(
                    Address([0u8; 20]),
                    Balance::from(1),
                )),
            ],
        };

        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.to_proto().encode_to_vec();
        let txs = vec![tx_bytes.into()];
        let (action_commitment, _) = generate_sequence_actions_commitment(txs);

        let expected_commitment: [u8; 32] = [
            233, 5, 49, 240, 176, 94, 136, 23, 160, 179, 175, 4, 63, 238, 60, 35, 250, 51, 255,
            150, 120, 169, 124, 85, 19, 36, 53, 120, 99, 177, 110, 8,
        ];
        assert_eq!(action_commitment, expected_commitment);
    }
}
