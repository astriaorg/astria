use std::collections::HashMap;

use astria_core::sequencer::{
    v1::{
        group_sequence_actions_in_signed_transaction_transactions_by_rollup_id,
        RollupId,
        SignedTransaction,
    },
    v2alpha1::block::{
        Deposit,
        RollupData,
    },
};
use bytes::Bytes;

/// Wrapper for values returned by [`generate_rollup_datas_commitment`].
pub(crate) struct GeneratedCommitments {
    pub(crate) rollup_datas_root: [u8; 32],
    pub(crate) rollup_ids_root: [u8; 32],
}

impl GeneratedCommitments {
    /// Converts the commitments plus external transaction data into a vector of bytes
    /// which can be used as the block's transactions.
    #[must_use]
    pub(crate) fn into_transactions(self, mut tx_data: Vec<Bytes>) -> Vec<Bytes> {
        let mut txs = Vec::with_capacity(tx_data.len() + 2);
        txs.push(self.rollup_datas_root.to_vec().into());
        txs.push(self.rollup_ids_root.to_vec().into());
        txs.append(&mut tx_data);
        txs
    }
}

/// Called when we receive a `PrepareProposal` or `ProcessProposal` consensus message.
///
/// In the case of `PrepareProposal`, we use this function to generate the `rollup_datas_commitment`
/// to be placed at the start of the block.
///
/// In the case of `ProcessProposal`, we use this function to generate and verify the
/// `rollup_datas_commitment` expected at the start of the block.
///
/// This function sorts the block's `sequence::Action`s contained within the transactions
/// using their `rollup_id`. It also appends the `Deposit`s generated by the block execution
/// to each rollup's sequenced data.
/// It then returns the merkle root of the tree where each leaf is
/// a commitment of the rollup data (`sequence::Action`s and `Deposit`s) with the same `rollup_id`.
/// The leaves are ordered by `rollup_id` in ascending order.
///
/// The leaf, which contains a commitment to every action with the same `rollup_id`, is currently
/// implemented as ( `rollup_id` || root of merkle tree of the `sequence::Action`s ).
/// This is somewhat arbitrary, but could be useful for proof of an action within the rollup datas
/// tree.
pub(crate) fn generate_rollup_datas_commitment(
    signed_txs: &[SignedTransaction],
    deposits: HashMap<RollupId, Vec<Deposit>>,
) -> GeneratedCommitments {
    use prost::Message as _;

    let mut rollup_ids_to_txs =
        group_sequence_actions_in_signed_transaction_transactions_by_rollup_id(signed_txs);

    for (rollup_id, deposit) in deposits {
        rollup_ids_to_txs.entry(rollup_id).or_default().extend(
            deposit
                .into_iter()
                .map(|deposit| RollupData::Deposit(deposit).into_raw().encode_to_vec()),
        );
    }

    rollup_ids_to_txs.sort_unstable_keys();
    let rollup_ids_root = merkle::Tree::from_leaves(rollup_ids_to_txs.keys()).root();

    // each leaf of the action tree is the root of a merkle tree of the `sequence::Action`s
    // with the same `rollup_id`, prepended with `rollup_id`.
    // the leaves are sorted in ascending order by `rollup_id`.
    let rollup_datas_root =
        astria_core::sequencer::v1::derive_merkle_tree_from_rollup_txs(&rollup_ids_to_txs).root();
    GeneratedCommitments {
        rollup_datas_root,
        rollup_ids_root,
    }
}

#[cfg(test)]
mod test {
    use astria_core::sequencer::v1::{
        asset::{
            Denom,
            DEFAULT_NATIVE_ASSET_DENOM,
        },
        transaction::action::{
            SequenceAction,
            TransferAction,
        },
        Address,
        UnsignedTransaction,
    };
    use ed25519_consensus::SigningKey;
    use rand::rngs::OsRng;

    use super::*;
    use crate::asset::{
        get_native_asset,
        NATIVE_ASSET,
    };

    #[test]
    fn generate_rollup_datas_commitment_should_ignore_transfers() {
        let _ = NATIVE_ASSET.set(Denom::from_base_denom(DEFAULT_NATIVE_ASSET_DENOM));

        let sequence_action = SequenceAction {
            rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
            data: b"helloworld".to_vec(),
            fee_asset_id: get_native_asset().id(),
        };
        let transfer_action = TransferAction {
            to: Address::from([0u8; 20]),
            amount: 1,
            asset_id: get_native_asset().id(),
            fee_asset_id: get_native_asset().id(),
        };

        let signing_key = SigningKey::new(OsRng);
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![sequence_action.clone().into(), transfer_action.into()],
        };

        let signed_tx = tx.into_signed(&signing_key);
        let txs = vec![signed_tx];
        let GeneratedCommitments {
            rollup_datas_root: commitment_0,
            ..
        } = generate_rollup_datas_commitment(&txs, HashMap::new());

        let signing_key = SigningKey::new(OsRng);
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![sequence_action.into()],
        };

        let signed_tx = tx.into_signed(&signing_key);
        let txs = vec![signed_tx];
        let GeneratedCommitments {
            rollup_datas_root: commitment_1,
            ..
        } = generate_rollup_datas_commitment(&txs, HashMap::new());
        assert_eq!(commitment_0, commitment_1);
    }

    #[test]
    // TODO(https://github.com/astriaorg/astria/issues/312): ensure this test is stable
    // against changes in the serialization format (protobuf is not deterministic)
    fn generate_rollup_datas_commitment_snapshot() {
        // this tests that the commitment generated is what is expected via a test vector.
        // this test will only break in the case of a breaking change to the commitment scheme,
        // thus if this test needs to be updated, we should cut a new release.
        let _ = NATIVE_ASSET.set(Denom::from_base_denom(DEFAULT_NATIVE_ASSET_DENOM));

        let sequence_action = SequenceAction {
            rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
            data: b"helloworld".to_vec(),
            fee_asset_id: get_native_asset().id(),
        };
        let transfer_action = TransferAction {
            to: Address::from([0u8; 20]),
            amount: 1,
            asset_id: get_native_asset().id(),
            fee_asset_id: get_native_asset().id(),
        };

        let signing_key = SigningKey::new(OsRng);
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![sequence_action.into(), transfer_action.into()],
        };

        let signed_tx = tx.into_signed(&signing_key);
        let txs = vec![signed_tx];
        let GeneratedCommitments {
            rollup_datas_root: actual,
            ..
        } = generate_rollup_datas_commitment(&txs, HashMap::new());

        let expected: [u8; 32] = [
            189, 156, 127, 228, 51, 249, 64, 237, 150, 91, 219, 216, 1, 99, 135, 28, 235, 15, 249,
            129, 3, 59, 231, 75, 92, 72, 103, 106, 173, 167, 251, 238,
        ];
        assert_eq!(expected, actual);
    }
}
