use astria_core::sequencer::v1alpha1::SignedTransaction;
use bytes::Bytes;

/// Wrapper for values returned by [`generate_sequence_actions_commitment`].
pub(crate) struct GeneratedCommitments {
    pub(crate) sequence_actions_root: [u8; 32],
    pub(crate) rollup_ids_root: [u8; 32],
}

impl GeneratedCommitments {
    /// Converts the commitments plus external transaction data into a vector of bytes
    /// which can be used as the block's transactions.
    #[must_use]
    pub(crate) fn into_transactions(self, mut tx_data: Vec<Bytes>) -> Vec<Bytes> {
        let mut txs = Vec::with_capacity(tx_data.len() + 2);
        txs.push(self.sequence_actions_root.to_vec().into());
        txs.push(self.rollup_ids_root.to_vec().into());
        txs.append(&mut tx_data);
        txs
    }
}

/// Called when we receive a `PrepareProposal` or `ProcessProposal` consensus message.
///
/// In the case of `PrepareProposal`, we use this function to generate the `commitment_tx`
/// to be placed at the start of the block.
///
/// In the case of `ProcessProposal`, we use this function to generate and verify the
/// `commitment_tx` expected at the start of the block.
///
/// This function sorts the block's `sequence::Action`s contained within the transactions
/// using their `rollup_id`. It then returns the merkle root of the tree where each leaf is
/// a commitment of `sequence::Action`s with the same `rollup_id`. The leaves are ordered
/// by `rollup_id` in ascending order.
/// This structure can be referred to as the "action tree".
///
/// The leaf, which contains a commitment to every action with the same `rollup_id`, is currently
/// implemented as ( `rollup_id` || root of merkle tree of the `sequence::Action`s ).
/// This is somewhat arbitrary, but could be useful for proof of an action within the action tree.
pub(crate) fn generate_sequence_actions_commitment(
    signed_txs: &[SignedTransaction],
) -> GeneratedCommitments {
    let rollup_ids_to_txs = astria_core::sequencer::v1alpha1::group_sequence_actions_in_signed_transaction_transactions_by_rollup_id(signed_txs);
    let rollup_ids_root = merkle::Tree::from_leaves(rollup_ids_to_txs.keys()).root();

    // each leaf of the action tree is the root of a merkle tree of the `sequence::Action`s
    // with the same `rollup_id`, prepended with `rollup_id`.
    // the leaves are sorted in ascending order by `rollup_id`.
    let sequence_actions_root =
        astria_core::sequencer::v1alpha1::derive_merkle_tree_from_rollup_txs(&rollup_ids_to_txs)
            .root();
    GeneratedCommitments {
        sequence_actions_root,
        rollup_ids_root,
    }
}

#[cfg(test)]
mod test {
    use astria_core::sequencer::v1alpha1::{
        asset::{
            Denom,
            DEFAULT_NATIVE_ASSET_DENOM,
        },
        transaction::action::{
            SequenceAction,
            TransferAction,
        },
        Address,
        RollupId,
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
    fn generate_sequence_actions_commitment_should_ignore_transfers() {
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
            sequence_actions_root: commitment_0,
            ..
        } = generate_sequence_actions_commitment(&txs);

        let signing_key = SigningKey::new(OsRng);
        let tx = UnsignedTransaction {
            nonce: 0,
            actions: vec![sequence_action.into()],
        };

        let signed_tx = tx.into_signed(&signing_key);
        let txs = vec![signed_tx];
        let GeneratedCommitments {
            sequence_actions_root: commitment_1,
            ..
        } = generate_sequence_actions_commitment(&txs);
        assert_eq!(commitment_0, commitment_1);
    }

    #[test]
    // TODO(https://github.com/astriaorg/astria/issues/312): ensure this test is stable
    // against changes in the serialization format (protobuf is not deterministic)
    fn generate_sequence_actions_commitment_snapshot() {
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
            sequence_actions_root: actual,
            ..
        } = generate_sequence_actions_commitment(&txs);

        let expected: [u8; 32] = [
            74, 113, 242, 162, 39, 84, 89, 175, 130, 76, 171, 61, 17, 189, 247, 101, 151, 181, 174,
            181, 52, 122, 131, 245, 56, 22, 11, 80, 217, 112, 44, 31,
        ];
        assert_eq!(expected, actual);
    }
}
