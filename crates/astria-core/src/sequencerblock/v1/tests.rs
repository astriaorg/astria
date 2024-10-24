use sha2::Digest as _;

use super::*;
use crate::protocol::transaction::test_utils::ConfigureSequencerBlock;

#[test]
fn sequencer_block_from_cometbft_block_gives_expected_merkle_proofs() {
    let sequencer_block = ConfigureSequencerBlock::default().make();
    let rollup_ids_root =
        merkle::Tree::from_leaves(sequencer_block.rollup_transactions.keys()).root();

    let rollup_transaction_tree = derive_merkle_tree_from_rollup_txs(
        sequencer_block
            .rollup_transactions
            .iter()
            .map(|(id, txs)| (id, txs.transactions())),
    );

    for rollup_transactions in sequencer_block.rollup_transactions.values() {
        assert!(
            super::super::do_rollup_transaction_match_root(
                rollup_transactions,
                rollup_transaction_tree.root()
            ),
            "audit failed; rollup transaction and its proof does not evaluate to rollup \
             transactions root",
        );
    }

    let data_hash: [u8; 32] = sequencer_block
        .header
        .cometbft_header
        .data_hash
        .unwrap()
        .as_bytes()
        .try_into()
        .unwrap();
    assert!(
        sequencer_block
            .rollup_transactions_proof
            .verify(&Sha256::digest(rollup_transaction_tree.root()), data_hash)
    );
    assert!(
        sequencer_block
            .rollup_ids_proof
            .verify(&Sha256::digest(rollup_ids_root), data_hash)
    );
}

#[test]
fn block_to_filtered_roundtrip() {
    let sequencer_block = ConfigureSequencerBlock::default().make();
    let rollup_ids = sequencer_block.rollup_transactions.keys();
    let filtered_sequencer_block = sequencer_block.to_filtered_block(rollup_ids);

    let raw = filtered_sequencer_block.clone().into_raw();
    let from_raw = FilteredSequencerBlock::try_from_raw(raw).unwrap();

    assert_eq!(filtered_sequencer_block, from_raw);
}
