use std::collections::BTreeMap;

use astria_sequencer_validation::generate_action_tree_leaves;
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
/// by `chain_id` in ascending order.
/// This structure can be referred to as the "action tree".
///
/// The leaf, which contains a commitment to every action with the same `chain_id`, is currently
/// implemented as ( `chain_id` || root of merkle tree of the `sequence::Action`s ).
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
                tracing::debug!(tx_bytes = hex::encode(tx_bytes), error = ?err, "failed to deserialize tx bytes");
                None
            }
        })
        .collect::<Vec<(Signed, Bytes)>>();
    let (signed_txs, txs_to_include): (Vec<Signed>, Vec<Bytes>) = txs.into_iter().unzip();

    let chain_id_to_txs = group_sequence_actions_by_chain_id(&signed_txs);

    // each leaf of the action tree is the root of a merkle tree of the `sequence::Action`s
    // with the same `chain_id`, prepended with `chain_id`.
    // the leaves are sorted in ascending order by `chain_id`.
    let leaves = generate_action_tree_leaves(chain_id_to_txs);
    (
        simple_hash_from_byte_vectors::<tendermint::crypto::default::Sha256>(&leaves),
        txs_to_include,
    )
}

/// Groups the `sequence::Action`s within the transactions by their `chain_id`.
/// Other types of actions are ignored.
///
/// Within an entry, actions are ordered by their transaction index within a block.
fn group_sequence_actions_by_chain_id(txs: &[Signed]) -> BTreeMap<Vec<u8>, Vec<Vec<u8>>> {
    let mut rollup_txs_map = BTreeMap::new();

    for tx in txs.iter() {
        tx.transaction().actions().iter().for_each(|action| {
            if let Some(action) = action.as_sequence() {
                let txs_for_rollup: &mut Vec<Vec<u8>> = rollup_txs_map
                    .entry(action.chain_id().to_vec())
                    .or_insert(vec![]);
                txs_for_rollup.push(action.data().to_vec());
            }
        });
    }

    rollup_txs_map
}

#[cfg(test)]
mod test {
    use astria_proto::native::sequencer::v1alpha1::Address;
    use ed25519_consensus::SigningKey;
    use prost::Message as _;
    use rand::rngs::OsRng;

    use super::*;
    use crate::{
        accounts::{
            self,
            types::{
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
    fn generate_action_tree_leaves_assert_leaves_ordered_by_chain_id() {
        let signing_key = SigningKey::new(OsRng);

        let chain_id_0 = b"testchainid0";
        let tx = Unsigned {
            nonce: Nonce::from(0),
            actions: vec![Action::SequenceAction(sequence::Action::new(
                chain_id_0.to_vec(),
                b"helloworld".to_vec(),
            ))],
        };
        let signed_tx_0 = tx.into_signed(&signing_key);

        let chain_id_1 = b"testchainid1";
        let tx = Unsigned {
            nonce: Nonce::from(0),
            actions: vec![Action::SequenceAction(sequence::Action::new(
                chain_id_1.to_vec(),
                b"helloworld".to_vec(),
            ))],
        };
        let signed_tx_1 = tx.into_signed(&signing_key);

        let chain_id_2 = b"testchainid2";
        let tx = Unsigned {
            nonce: Nonce::from(0),
            actions: vec![Action::SequenceAction(sequence::Action::new(
                chain_id_2.to_vec(),
                b"helloworld".to_vec(),
            ))],
        };
        let signed_tx_2 = tx.into_signed(&signing_key);

        let txs = vec![signed_tx_0, signed_tx_1, signed_tx_2];
        let chain_id_to_txs = group_sequence_actions_by_chain_id(&txs);
        let leaves = generate_action_tree_leaves(chain_id_to_txs);
        leaves.iter().enumerate().for_each(|(i, leaf)| {
            if i == 0 {
                return;
            }
            assert!(leaf > &leaves[i - 1]);
        });
    }

    #[test]
    fn generate_sequence_actions_commitment_snapshot() {
        // this tests that the commitment generated is what is expected via a test vector.
        // this test will only break in the case of a breaking change to the commitment scheme,
        // thus if this test needs to be updated, we should cut a new release.

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
            97, 82, 159, 138, 201, 12, 241, 95, 99, 19, 162, 205, 37, 38, 130, 165, 78, 185, 141,
            6, 69, 51, 32, 9, 224, 92, 34, 25, 192, 213, 235, 3,
        ];
        assert_eq!(action_commitment, expected_commitment);
    }
}
