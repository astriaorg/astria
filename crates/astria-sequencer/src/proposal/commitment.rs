use std::collections::BTreeMap;

use bytes::Bytes;
use proto::native::sequencer::v1alpha1::SignedTransaction;

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
    use proto::{
        generated::sequencer::v1alpha1 as raw,
        Message as _,
    };
    use sequencer_validation::generate_action_tree_leaves;
    use tendermint::{
        crypto::default::Sha256,
        merkle::simple_hash_from_byte_vectors,
    };
    use tracing::debug;

    let (signed_txs, txs_to_include): (Vec<_>, Vec<_>) = txs_bytes
        .into_iter()
        .filter_map(|bytes| {
            raw::SignedTransaction::decode(&*bytes)
            .map_err(|err| {
                debug!(error = ?err, "failed to deserialize bytes as a signed transaction");
                err
            })
            .ok()
            .and_then(|raw_tx| SignedTransaction::try_from_raw(raw_tx)
                .map_err(|err| {
                    debug!(error = ?err, "could not convert raw signed transaction to native signed transaction");
                    err
                })
                .ok()
            )
            .map(move |signed_tx| (signed_tx, bytes))
        })
        .unzip();

    let chain_id_to_txs = group_sequence_actions_by_chain_id(&signed_txs);

    // each leaf of the action tree is the root of a merkle tree of the `sequence::Action`s
    // with the same `chain_id`, prepended with `chain_id`.
    // the leaves are sorted in ascending order by `chain_id`.
    let leaves = generate_action_tree_leaves(chain_id_to_txs);
    (
        simple_hash_from_byte_vectors::<Sha256>(&leaves),
        txs_to_include,
    )
}

/// Groups the `sequence::Action`s within the transactions by their `chain_id`.
/// Other types of actions are ignored.
///
/// Within an entry, actions are ordered by their transaction index within a block.
fn group_sequence_actions_by_chain_id(
    txs: &[SignedTransaction],
) -> BTreeMap<Vec<u8>, Vec<Vec<u8>>> {
    let mut rollup_txs_map = BTreeMap::new();

    for action in txs.iter().flat_map(SignedTransaction::actions) {
        if let Some(action) = action.as_sequence() {
            let txs_for_rollup: &mut Vec<Vec<u8>> = rollup_txs_map
                .entry(action.chain_id.clone())
                .or_insert(vec![]);
            txs_for_rollup.push(action.data.clone());
        }
    }

    rollup_txs_map
}

#[cfg(test)]
mod test {
    use ed25519_consensus::SigningKey;
    use proto::{
        native::sequencer::v1alpha1::{
            Address,
            SequenceAction,
            TransferAction,
            UnsignedTransaction,
        },
        Message as _,
    };
    use rand::rngs::OsRng;
    use sequencer_validation::generate_action_tree_leaves;

    use super::*;

    #[test]
    fn generate_sequence_actions_commitment_should_ignore_transfers() {
        let sequence_action = SequenceAction {
            chain_id: b"testchainid".to_vec(),
            data: b"helloworld".to_vec(),
        };
        let transfer_action = TransferAction {
            to: Address([0u8; 20]),
            amount: 1,
        };

        let signing_key = SigningKey::new(OsRng);
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![sequence_action.clone().into(), transfer_action.into()],
        };

        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.into_raw().encode_to_vec();
        let txs = vec![tx_bytes.into()];
        let (action_commitment_0, _) = generate_sequence_actions_commitment(txs);

        let signing_key = SigningKey::new(OsRng);
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![sequence_action.into()],
        };

        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.into_raw().encode_to_vec();
        let txs = vec![tx_bytes.into()];
        let (action_commitment_1, _) = generate_sequence_actions_commitment(txs);
        assert_eq!(action_commitment_0, action_commitment_1);
    }

    #[test]
    fn generate_action_tree_leaves_assert_leaves_ordered_by_chain_id() {
        let signing_key = SigningKey::new(OsRng);

        let chain_id_0 = b"testchainid0";
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                SequenceAction {
                    chain_id: chain_id_0.to_vec(),
                    data: b"helloworld".to_vec(),
                }
                .into(),
            ],
        };
        let signed_tx_0 = tx.into_signed(&signing_key);

        let chain_id_1 = b"testchainid1";
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                SequenceAction {
                    chain_id: chain_id_1.to_vec(),
                    data: b"helloworld".to_vec(),
                }
                .into(),
            ],
        };
        let signed_tx_1 = tx.into_signed(&signing_key);

        let chain_id_2 = b"testchainid2";
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![
                SequenceAction {
                    chain_id: chain_id_2.to_vec(),
                    data: b"helloworld".to_vec(),
                }
                .into(),
            ],
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
    // TODO(https://github.com/astriaorg/astria/issues/312): ensure this test is stable
    // against changes in the serialization format (protobuf is not deterministic)
    fn generate_sequence_actions_commitment_snapshot() {
        // this tests that the commitment generated is what is expected via a test vector.
        // this test will only break in the case of a breaking change to the commitment scheme,
        // thus if this test needs to be updated, we should cut a new release.

        let sequence_action = SequenceAction {
            chain_id: b"testchainid".to_vec(),
            data: b"helloworld".to_vec(),
        };
        let transfer_action = TransferAction {
            to: Address([0u8; 20]),
            amount: 1,
        };

        let signing_key = SigningKey::new(OsRng);
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![sequence_action.into(), transfer_action.into()],
        };

        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.into_raw().encode_to_vec();
        let txs = vec![tx_bytes.into()];
        let (actual, _) = generate_sequence_actions_commitment(txs);

        let expected: [u8; 32] = [
            97, 82, 159, 138, 201, 12, 241, 95, 99, 19, 162, 205, 37, 38, 130, 165, 78, 185, 141,
            6, 69, 51, 32, 9, 224, 92, 34, 25, 192, 213, 235, 3,
        ];
        assert_eq!(expected, actual);
    }
}
