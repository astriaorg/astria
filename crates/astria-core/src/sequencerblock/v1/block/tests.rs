use astria_core_address::Address;
use bytes::Bytes;
use prost::Message as _;
use sha2::Digest as _;

use super::{
    super::do_rollup_transactions_match_root,
    *,
};
use crate::{
    crypto::SigningKey,
    generated::protocol::transaction::v1::Transaction as RawTransaction,
    protocol::{
        test_utils::{
            minimal_extended_commit_info,
            minimal_extended_commit_info_bytes,
            upgrade_change_hashes,
            upgrade_change_hashes_bytes,
            ConfigureSequencerBlock,
        },
        transaction::v1::{
            action::Transfer,
            Transaction,
            TransactionBody,
        },
    },
    Protobuf as _,
};

const ROLLUP_TXS_ROOT: [u8; 32] = [1; 32];
const ROLLUP_IDS_ROOT: [u8; 32] = [2; 32];

#[test]
fn sequencer_block_from_cometbft_block_gives_expected_merkle_proofs() {
    let sequencer_block = ConfigureSequencerBlock::default().make();
    let rollup_ids_root =
        merkle::Tree::from_leaves(sequencer_block.rollup_transactions().keys()).root();

    let rollup_transaction_tree = derive_merkle_tree_from_rollup_txs(
        sequencer_block
            .rollup_transactions()
            .iter()
            .map(|(id, txs)| (id, txs.transactions())),
    );

    for rollup_transactions in sequencer_block.rollup_transactions().values() {
        assert!(
            do_rollup_transactions_match_root(rollup_transactions, rollup_transaction_tree.root()),
            "audit failed; rollup transaction and its proof does not evaluate to rollup \
             transactions root",
        );
    }

    let data_hash = *sequencer_block.header().data_hash();
    assert!(sequencer_block
        .rollup_transactions_proof()
        .verify(&Sha256::digest(rollup_transaction_tree.root()), data_hash));
    assert!(sequencer_block
        .rollup_ids_proof()
        .verify(&Sha256::digest(rollup_ids_root), data_hash));
}

#[test]
fn block_to_filtered_roundtrip() {
    let sequencer_block = ConfigureSequencerBlock::default().make();
    let rollup_ids = sequencer_block.rollup_transactions().keys();
    let filtered_sequencer_block = sequencer_block.to_filtered_block(rollup_ids);

    let raw = filtered_sequencer_block.clone().into_raw();
    let from_raw = FilteredSequencerBlock::try_from_raw(raw).unwrap();

    assert_eq!(filtered_sequencer_block, from_raw);
}

#[test]
fn encoded_rollup_txs_root_length_should_be_correct() {
    assert_eq!(
        DataItem::RollupTransactionsRoot([1; 32]).encode().len(),
        DataItem::ENCODED_ROLLUP_TRANSACTIONS_ROOT_LENGTH
    );
}

#[test]
fn encoded_rollup_ids_root_length_should_be_correct() {
    assert_eq!(
        DataItem::RollupIdsRoot([1; 32]).encode().len(),
        DataItem::ENCODED_ROLLUP_IDS_ROOT_LENGTH
    );
}

fn rollup_txs_root_legacy_bytes() -> Bytes {
    ROLLUP_TXS_ROOT.as_slice().into()
}

fn rollup_txs_root_bytes() -> Bytes {
    DataItem::RollupTransactionsRoot(ROLLUP_TXS_ROOT).encode()
}

fn rollup_ids_root_legacy_bytes() -> Bytes {
    ROLLUP_IDS_ROOT.as_slice().into()
}

fn rollup_ids_root_bytes() -> Bytes {
    DataItem::RollupIdsRoot(ROLLUP_IDS_ROOT).encode()
}

fn tx(nonce: u32) -> Transaction {
    let signing_key = SigningKey::from([
        213, 191, 74, 63, 204, 231, 23, 176, 56, 139, 204, 39, 73, 235, 193, 72, 173, 153, 105,
        178, 63, 69, 238, 27, 96, 95, 213, 135, 120, 87, 106, 196,
    ]);

    let transfer = Transfer {
        to: Address::builder()
            .array([0; 20])
            .prefix("astria")
            .try_build()
            .unwrap(),
        amount: 0,
        asset: "nria".parse().unwrap(),
        fee_asset: "nria".parse().unwrap(),
    };

    let body = TransactionBody::builder()
        .actions(vec![transfer.into()])
        .chain_id("test-1".to_string())
        .nonce(nonce)
        .try_build()
        .unwrap();

    body.sign(&signing_key)
}

fn tx_bytes(nonce: u32) -> Bytes {
    tx(nonce).into_raw().encode_to_vec().into()
}

macro_rules! assert_err_matches {
    ($expression:expr, $pattern:pat $(if $guard:expr)? $(,)?) => {
        let pattern_str = stringify!($pattern);
        let if_guard_str = stringify!($(if $guard)?);
        let expected_str = if if_guard_str.is_empty() {
            format!("`{pattern_str}`")
        } else {
            format!("`{pattern_str} {if_guard_str}`")
        };
        let expected = expected_str.replace('\n', " ");
        assert!(
            matches!($expression, $pattern $(if $guard)?),
            "expected {expected}, got `SequencerBlockErrorKind::{:?}`", $expression,
        );
    };
}

// Tests for the `ExpandedBlockData` constructors.
mod expanded_block_data {
    use super::*;

    #[test]
    fn should_fail_to_parse_legacy_txs_missing_rollup_txs_root() {
        let data = [];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_untyped_data(&data).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::NoRollupTransactionsRoot
        );
    }

    #[test]
    fn should_fail_to_parse_legacy_txs_malformed_rollup_txs_root() {
        let data = [vec![0; 31].into()];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_untyped_data(&data).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::IncorrectRollupTransactionsRootLength
        );
    }

    #[test]
    fn should_fail_to_parse_legacy_txs_missing_rollup_ids_root() {
        let data = [rollup_txs_root_legacy_bytes()];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_untyped_data(&data).unwrap_err();
        assert_err_matches!(error_kind, SequencerBlockErrorKind::NoRollupIdsRoot);
    }

    #[test]
    fn should_fail_to_parse_legacy_txs_malformed_rollup_ids_root() {
        let data = [rollup_txs_root_legacy_bytes(), vec![0; 31].into()];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_untyped_data(&data).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::IncorrectRollupIdsRootLength
        );
    }

    #[test]
    fn should_fail_to_parse_legacy_txs_malformed_protobuf_tx() {
        let data = [
            rollup_txs_root_legacy_bytes(),
            rollup_ids_root_legacy_bytes(),
            vec![3].into(),
        ];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_untyped_data(&data).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::TransactionProtobufDecode(_)
        );
    }

    #[test]
    fn should_fail_to_parse_legacy_txs_malformed_raw_tx() {
        let bad_tx = RawTransaction {
            signature: vec![1].into(),
            public_key: vec![2].into(),
            body: None,
        };
        let data = [
            rollup_txs_root_legacy_bytes(),
            rollup_ids_root_legacy_bytes(),
            bad_tx.encode_to_vec().into(),
        ];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_untyped_data(&data).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::RawTransactionConversion(_)
        );
    }

    #[test]
    fn should_parse_legacy_txs_with_user_submitted_txs() {
        let data = [
            rollup_txs_root_legacy_bytes(),
            rollup_ids_root_legacy_bytes(),
            tx_bytes(0),
            tx_bytes(1),
            tx_bytes(2),
        ];
        let items = ExpandedBlockData::new_from_untyped_data(&data).unwrap();
        assert_eq!(ROLLUP_TXS_ROOT, items.rollup_transactions_root);
        assert_eq!(ROLLUP_IDS_ROOT, items.rollup_ids_root);
        assert_eq!(tx(0).id(), items.user_submitted_transactions[0].id());
        assert_eq!(tx(1).id(), items.user_submitted_transactions[1].id());
        assert_eq!(tx(2).id(), items.user_submitted_transactions[2].id());
    }

    #[test]
    fn should_parse_legacy_txs_with_no_user_submitted_txs() {
        let data = [
            rollup_txs_root_legacy_bytes(),
            rollup_ids_root_legacy_bytes(),
        ];
        let items = ExpandedBlockData::new_from_untyped_data(&data).unwrap();
        assert_eq!(ROLLUP_TXS_ROOT, items.rollup_transactions_root);
        assert_eq!(ROLLUP_IDS_ROOT, items.rollup_ids_root);
        assert!(items.upgrade_change_hashes.is_empty());
        assert!(items.extended_commit_info_with_proof.is_none());
        assert!(items.user_submitted_transactions.is_empty());
    }

    #[test]
    fn should_fail_to_parse_data_items_missing_rollup_txs_root() {
        let data = [];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_typed_data(&data, true).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::NoRollupTransactionsRoot
        );
    }

    #[test]
    fn should_fail_to_parse_data_items_malformed_item() {
        let data = [vec![0; 31].into()];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_typed_data(&data, true).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::DataItem(DataItemError(DataItemErrorKind::Decode { .. }))
        );
    }

    #[test]
    fn should_fail_to_parse_data_items_rollup_txs_root_not_first_item() {
        let data = [rollup_ids_root_bytes(), rollup_txs_root_bytes()];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_typed_data(&data, true).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::DataItem(DataItemError(DataItemErrorKind::Mismatch { index, .. }))
            if index == 0
        );
    }

    #[test]
    fn should_fail_to_parse_data_items_missing_rollup_ids_root() {
        let data = [rollup_txs_root_bytes()];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_typed_data(&data, true).unwrap_err();
        assert_err_matches!(error_kind, SequencerBlockErrorKind::NoRollupIdsRoot);
    }

    #[test]
    fn should_fail_to_parse_data_items_rollup_ids_root_not_second_item() {
        let data = [rollup_txs_root_bytes(), rollup_txs_root_bytes()];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_typed_data(&data, true).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::DataItem(DataItemError(DataItemErrorKind::Mismatch { index, .. }))
            if index == 1
        );
    }

    #[test]
    fn should_fail_to_parse_data_items_missing_required_extended_commit_info() {
        let data = [rollup_txs_root_bytes(), rollup_ids_root_bytes()];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_typed_data(&data, true).unwrap_err();
        assert_err_matches!(error_kind, SequencerBlockErrorKind::NoExtendedCommitInfo);
    }

    #[test]
    fn should_fail_to_parse_data_items_malformed_protobuf_tx() {
        let data = [
            rollup_txs_root_bytes(),
            rollup_ids_root_bytes(),
            minimal_extended_commit_info_bytes(),
            vec![3].into(),
        ];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_typed_data(&data, true).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::TransactionProtobufDecode(_)
        );
    }

    #[test]
    fn should_fail_to_parse_data_items_malformed_raw_tx() {
        let bad_tx = RawTransaction {
            signature: vec![1].into(),
            public_key: vec![2].into(),
            body: None,
        };
        let data = [
            rollup_txs_root_bytes(),
            rollup_ids_root_bytes(),
            minimal_extended_commit_info_bytes(),
            bad_tx.encode_to_vec().into(),
        ];
        let SequencerBlockError(error_kind) =
            ExpandedBlockData::new_from_typed_data(&data, true).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::RawTransactionConversion(_)
        );
    }

    #[test]
    fn should_parse_with_no_upgrade_change_hashes_no_extended_commit_info_no_user_submitted_txs() {
        let data = [rollup_txs_root_bytes(), rollup_ids_root_bytes()];
        let items = ExpandedBlockData::new_from_typed_data(&data, false).unwrap();
        assert_eq!(ROLLUP_TXS_ROOT, items.rollup_transactions_root);
        assert_eq!(ROLLUP_IDS_ROOT, items.rollup_ids_root);
        assert!(items.upgrade_change_hashes.is_empty());
        assert!(items.extended_commit_info_with_proof.is_none());
        assert!(items.user_submitted_transactions.is_empty());
    }

    #[test]
    fn should_parse_with_no_upgrade_change_hashes_no_extended_commit_info_some_user_submitted_txs()
    {
        let data = [
            rollup_txs_root_bytes(),
            rollup_ids_root_bytes(),
            tx_bytes(0),
            tx_bytes(1),
            tx_bytes(2),
        ];
        let items = ExpandedBlockData::new_from_typed_data(&data, false).unwrap();
        assert_eq!(ROLLUP_TXS_ROOT, items.rollup_transactions_root);
        assert_eq!(ROLLUP_IDS_ROOT, items.rollup_ids_root);
        assert!(items.upgrade_change_hashes.is_empty());
        assert!(items.extended_commit_info_with_proof.is_none());
        assert_eq!(tx(0).id(), items.user_submitted_transactions[0].id());
        assert_eq!(tx(1).id(), items.user_submitted_transactions[1].id());
        assert_eq!(tx(2).id(), items.user_submitted_transactions[2].id());
    }

    #[test]
    fn should_parse_with_no_upgrade_change_hashes_some_extended_commit_info_no_user_submitted_txs()
    {
        let data = [
            rollup_txs_root_bytes(),
            rollup_ids_root_bytes(),
            minimal_extended_commit_info_bytes(),
        ];
        let items = ExpandedBlockData::new_from_typed_data(&data, true).unwrap();
        assert_eq!(ROLLUP_TXS_ROOT, items.rollup_transactions_root);
        assert_eq!(ROLLUP_IDS_ROOT, items.rollup_ids_root);
        assert!(items.upgrade_change_hashes.is_empty());
        assert_eq!(
            &minimal_extended_commit_info(),
            items
                .extended_commit_info_with_proof
                .unwrap()
                .extended_commit_info()
        );
        assert!(items.user_submitted_transactions.is_empty());
    }

    #[test]
    fn should_parse_with_no_upgrade_change_hashes_some_extended_commit_info_some_user_submitted_txs(
    ) {
        let data = [
            rollup_txs_root_bytes(),
            rollup_ids_root_bytes(),
            minimal_extended_commit_info_bytes(),
            tx_bytes(0),
            tx_bytes(1),
            tx_bytes(2),
        ];
        let items = ExpandedBlockData::new_from_typed_data(&data, true).unwrap();
        assert_eq!(ROLLUP_TXS_ROOT, items.rollup_transactions_root);
        assert_eq!(ROLLUP_IDS_ROOT, items.rollup_ids_root);
        assert!(items.upgrade_change_hashes.is_empty());
        assert_eq!(
            &minimal_extended_commit_info(),
            items
                .extended_commit_info_with_proof
                .unwrap()
                .extended_commit_info()
        );
        assert_eq!(tx(0).id(), items.user_submitted_transactions[0].id());
        assert_eq!(tx(1).id(), items.user_submitted_transactions[1].id());
        assert_eq!(tx(2).id(), items.user_submitted_transactions[2].id());
    }

    #[test]
    fn should_parse_with_upgrade_change_hashes_no_extended_commit_info_no_user_submitted_txs() {
        let data = [
            rollup_txs_root_bytes(),
            rollup_ids_root_bytes(),
            upgrade_change_hashes_bytes(),
        ];
        let items = ExpandedBlockData::new_from_typed_data(&data, false).unwrap();
        assert_eq!(ROLLUP_TXS_ROOT, items.rollup_transactions_root);
        assert_eq!(ROLLUP_IDS_ROOT, items.rollup_ids_root);
        assert_eq!(upgrade_change_hashes(), items.upgrade_change_hashes);
        assert!(items.extended_commit_info_with_proof.is_none());
        assert!(items.user_submitted_transactions.is_empty());
    }

    #[test]
    fn should_parse_with_upgrade_change_hashes_no_extended_commit_info_some_user_submitted_txs() {
        let data = [
            rollup_txs_root_bytes(),
            rollup_ids_root_bytes(),
            upgrade_change_hashes_bytes(),
            tx_bytes(0),
            tx_bytes(1),
            tx_bytes(2),
        ];
        let items = ExpandedBlockData::new_from_typed_data(&data, false).unwrap();
        assert_eq!(ROLLUP_TXS_ROOT, items.rollup_transactions_root);
        assert_eq!(ROLLUP_IDS_ROOT, items.rollup_ids_root);
        assert_eq!(upgrade_change_hashes(), items.upgrade_change_hashes);
        assert!(items.extended_commit_info_with_proof.is_none());
        assert_eq!(tx(0).id(), items.user_submitted_transactions[0].id());
        assert_eq!(tx(1).id(), items.user_submitted_transactions[1].id());
        assert_eq!(tx(2).id(), items.user_submitted_transactions[2].id());
    }

    #[test]
    fn should_parse_with_upgrade_change_hashes_some_extended_commit_info_no_user_submitted_txs() {
        let data = [
            rollup_txs_root_bytes(),
            rollup_ids_root_bytes(),
            upgrade_change_hashes_bytes(),
            minimal_extended_commit_info_bytes(),
        ];
        let items = ExpandedBlockData::new_from_typed_data(&data, true).unwrap();
        assert_eq!(ROLLUP_TXS_ROOT, items.rollup_transactions_root);
        assert_eq!(ROLLUP_IDS_ROOT, items.rollup_ids_root);
        assert_eq!(upgrade_change_hashes(), items.upgrade_change_hashes);
        assert_eq!(
            &minimal_extended_commit_info(),
            items
                .extended_commit_info_with_proof
                .unwrap()
                .extended_commit_info()
        );
        assert!(items.user_submitted_transactions.is_empty());
    }

    #[test]
    fn should_parse_with_upgrade_change_hashes_some_extended_commit_info_some_user_submitted_txs() {
        let data = [
            rollup_txs_root_bytes(),
            rollup_ids_root_bytes(),
            upgrade_change_hashes_bytes(),
            minimal_extended_commit_info_bytes(),
            tx_bytes(0),
            tx_bytes(1),
            tx_bytes(2),
        ];
        let items = ExpandedBlockData::new_from_typed_data(&data, true).unwrap();
        assert_eq!(ROLLUP_TXS_ROOT, items.rollup_transactions_root);
        assert_eq!(ROLLUP_IDS_ROOT, items.rollup_ids_root);
        assert_eq!(upgrade_change_hashes(), items.upgrade_change_hashes);
        assert_eq!(
            &minimal_extended_commit_info(),
            items
                .extended_commit_info_with_proof
                .unwrap()
                .extended_commit_info()
        );
        assert_eq!(tx(0).id(), items.user_submitted_transactions[0].id());
        assert_eq!(tx(1).id(), items.user_submitted_transactions[1].id());
        assert_eq!(tx(2).id(), items.user_submitted_transactions[2].id());
    }
}

// Tests for nontrivial parts of `try_from_raw` constructors.
mod try_from_raw {
    use super::*;

    fn rollup_1() -> RollupId {
        RollupId::from_unhashed_bytes(b"rollup_1")
    }

    fn rollup_2() -> RollupId {
        RollupId::from_unhashed_bytes(b"rollup_2")
    }

    /// Three txs all from rollup 1.
    fn block_with_three_txs() -> SequencerBlock {
        ConfigureSequencerBlock {
            sequence_data: vec![
                (rollup_1(), vec![1]),
                (rollup_1(), vec![2]),
                (rollup_1(), vec![3]),
            ],
            ..ConfigureSequencerBlock::default()
        }
        .make()
    }

    /// Three txs from rollup 1 and three from rollup 2.
    fn block_with_six_txs() -> SequencerBlock {
        ConfigureSequencerBlock {
            sequence_data: vec![
                (rollup_1(), vec![1]),
                (rollup_1(), vec![2]),
                (rollup_1(), vec![3]),
                (rollup_2(), vec![4]),
                (rollup_2(), vec![5]),
                (rollup_2(), vec![6]),
            ],
            ..ConfigureSequencerBlock::default()
        }
        .make()
    }

    #[test]
    fn sequencer_block_should_fail_bad_rollup_txs_proof() {
        let mut block = block_with_six_txs().into_raw();
        block.rollup_transactions_proof =
            block_with_three_txs().into_raw().rollup_transactions_proof;
        let SequencerBlockError(error_kind) = SequencerBlock::try_from_raw(block).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::InvalidRollupTransactionsRoot
        );
    }

    #[test]
    fn sequencer_block_should_fail_missing_rollup_tx() {
        let mut block = block_with_six_txs().into_raw();
        // Pop the last tx from rollup 2's txs.
        let _ = block
            .rollup_transactions
            .last_mut()
            .unwrap()
            .transactions
            .pop();
        let SequencerBlockError(error_kind) = SequencerBlock::try_from_raw(block).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::RollupTransactionsNotInSequencerBlock
        );
    }

    #[test]
    fn sequencer_block_should_fail_extra_rollup_tx() {
        let mut block = block_with_six_txs().into_raw();
        // Push a tx to rollup 2's txs.
        block
            .rollup_transactions
            .last_mut()
            .unwrap()
            .transactions
            .push(vec![4].into());
        let SequencerBlockError(error_kind) = SequencerBlock::try_from_raw(block).unwrap_err();
        assert_err_matches!(
            error_kind,
            SequencerBlockErrorKind::RollupTransactionsNotInSequencerBlock
        );
    }

    #[test]
    fn sequencer_block_should_fail_bad_rollup_ids_proof() {
        let mut block = block_with_six_txs().into_raw();
        block.rollup_ids_proof = block_with_three_txs().into_raw().rollup_ids_proof;
        let SequencerBlockError(error_kind) = SequencerBlock::try_from_raw(block).unwrap_err();
        assert_err_matches!(error_kind, SequencerBlockErrorKind::InvalidRollupIdsProof);
    }

    #[test]
    fn filtered_sequencer_block_should_fail_bad_rollup_txs_proof() {
        let mut block = block_with_six_txs()
            .into_filtered_block([rollup_1(), rollup_2()])
            .into_raw();
        block.rollup_transactions_proof =
            block_with_three_txs().into_raw().rollup_transactions_proof;
        let FilteredSequencerBlockError(error_kind) =
            FilteredSequencerBlock::try_from_raw(block).unwrap_err();
        assert_err_matches!(
            error_kind,
            FilteredSequencerBlockErrorKind::RollupTransactionsNotInSequencerBlock
        );
    }

    #[test]
    fn filtered_sequencer_block_should_fail_missing_rollup_tx() {
        let mut block = block_with_six_txs()
            .into_filtered_block([rollup_1(), rollup_2()])
            .into_raw();
        // Pop the last tx from rollup 2's txs.
        let _ = block
            .rollup_transactions
            .last_mut()
            .unwrap()
            .transactions
            .pop();
        let FilteredSequencerBlockError(error_kind) =
            FilteredSequencerBlock::try_from_raw(block).unwrap_err();
        assert_err_matches!(
            error_kind,
            FilteredSequencerBlockErrorKind::RollupTransactionForIdNotInSequencerBlock { .. }
        );
    }

    #[test]
    fn filtered_sequencer_block_should_fail_extra_rollup_tx() {
        let mut block = block_with_six_txs()
            .into_filtered_block([rollup_1(), rollup_2()])
            .into_raw();
        // Push a tx to rollup 2's txs.
        block
            .rollup_transactions
            .last_mut()
            .unwrap()
            .transactions
            .push(vec![4].into());
        let FilteredSequencerBlockError(error_kind) =
            FilteredSequencerBlock::try_from_raw(block).unwrap_err();
        assert_err_matches!(
            error_kind,
            FilteredSequencerBlockErrorKind::RollupTransactionForIdNotInSequencerBlock { .. }
        );
    }

    #[test]
    fn filtered_sequencer_block_should_fail_bad_rollup_ids_proof() {
        let mut block = block_with_six_txs()
            .into_filtered_block([rollup_1(), rollup_2()])
            .into_raw();
        block.rollup_ids_proof = block_with_three_txs().into_raw().rollup_ids_proof;
        let FilteredSequencerBlockError(error_kind) =
            FilteredSequencerBlock::try_from_raw(block).unwrap_err();
        assert_err_matches!(
            error_kind,
            FilteredSequencerBlockErrorKind::InvalidRollupIdsProof
        );
    }

    #[test]
    fn extended_commit_info_with_proof_should_fail_bad_proof() {
        let data = [
            rollup_txs_root_bytes(),
            rollup_ids_root_bytes(),
            minimal_extended_commit_info_bytes(),
        ];
        let items = ExpandedBlockData::new_from_typed_data(&data, true).unwrap();
        let mut extended_commit_info_with_proof =
            items.extended_commit_info_with_proof.unwrap().into_raw();

        // Change the proof to be invalid.
        extended_commit_info_with_proof
            .proof
            .as_mut()
            .unwrap()
            .leaf_index = 0;

        let ExtendedCommitInfoError(error_kind) = ExtendedCommitInfoWithProof::try_from_raw(
            extended_commit_info_with_proof,
            items.data_root_hash,
        )
        .unwrap_err();
        assert_err_matches!(error_kind, ExtendedCommitInfoErrorKind::NotInSequencerBlock);
    }
}
