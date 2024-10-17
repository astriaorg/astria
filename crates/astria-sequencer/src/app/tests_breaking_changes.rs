//! The tests in this file are snapshot tests that are expected to break
//! if breaking changes are made to the application, in particular,
//! any changes that can affect the state tree and thus the app hash.
//!
//! If these tests break due to snapshot mismatches, you can update the snapshots
//! with `cargo insta review`, but you MUST mark the respective PR as breaking.
//!
//! Note: there are two actions not tested here: `Ics20Withdrawal` and `IbcRelay`.
//! These are due to the extensive setup needed to test them.
//! If changes are made to the execution results of these actions, manual testing is required.

use std::{
    collections::HashMap,
    sync::Arc,
};

use astria_core::{
    primitive::v1::RollupId,
    protocol::{
        genesis::v1::Account,
        transaction::v1::{
            action::{
                BridgeLock,
                BridgeSudoChange,
                BridgeUnlock,
                IbcRelayerChange,
                IbcSudoChange,
                RollupDataSubmission,
                Transfer,
                ValidatorUpdate,
            },
            Action,
            TransactionBody,
        },
    },
    sequencerblock::v1::block::Deposit,
    Protobuf,
};
use cnidarium::StateDelta;
use prost::{
    bytes::Bytes,
    Message as _,
};
use tendermint::{
    abci,
    abci::types::CommitInfo,
    block::Round,
    Hash,
    Time,
};

use crate::{
    app::{
        benchmark_and_test_utils::{
            default_genesis_accounts,
            initialize_app_with_storage,
            proto_genesis_state,
            BOB_ADDRESS,
            CAROL_ADDRESS,
        },
        test_utils::{
            get_alice_signing_key,
            get_bridge_signing_key,
            initialize_app,
        },
    },
    authority::StateReadExt as _,
    benchmark_and_test_utils::{
        astria_address,
        astria_address_from_hex_string,
        nria,
        ASTRIA_PREFIX,
    },
    bridge::StateWriteExt as _,
    proposal::commitment::generate_rollup_datas_commitment,
};

#[tokio::test]
async fn app_genesis_snapshot() {
    let app = initialize_app(None, vec![]).await;
    insta::assert_json_snapshot!(app.app_hash.as_bytes());
}

#[tokio::test]
async fn app_finalize_block_snapshot() {
    let alice = get_alice_signing_key();
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

    let bridge_address = astria_address(&[99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let starting_index_of_action = 0;

    let mut delta_delta = StateDelta::new(app.state_delta.clone());
    delta_delta
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    delta_delta
        .put_bridge_account_ibc_asset(&bridge_address, nria())
        .unwrap();
    app.apply(delta_delta);

    // the state changes must be committed, as `finalize_block` will execute the
    // changes on the latest snapshot, not the app's `StateDelta`.
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    let amount = 100;
    let lock_action = BridgeLock {
        to: bridge_address,
        amount,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
    };
    let rollup_data_submission = RollupDataSubmission {
        rollup_id,
        data: Bytes::from_static(b"hello world"),
        fee_asset: nria().into(),
    };

    let tx = TransactionBody::builder()
        .actions(vec![lock_action.into(), rollup_data_submission.into()])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = tx.sign(&alice);

    let expected_deposit = Deposit {
        bridge_address,
        rollup_id,
        amount,
        asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
        source_transaction_id: signed_tx.id(),
        source_action_index: starting_index_of_action,
    };
    let deposits = HashMap::from_iter(vec![(rollup_id, vec![expected_deposit.clone()])]);
    let commitments = generate_rollup_datas_commitment(&[signed_tx.clone()], deposits.clone());

    let timestamp = Time::unix_epoch();
    let block_hash = Hash::try_from([99u8; 32].to_vec()).unwrap();
    let finalize_block = abci::request::FinalizeBlock {
        hash: block_hash,
        height: 1u32.into(),
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: commitments.into_transactions(vec![signed_tx.to_raw().encode_to_vec().into()]),
        decided_last_commit: CommitInfo {
            votes: vec![],
            round: Round::default(),
        },
        misbehavior: vec![],
    };

    app.finalize_block(finalize_block.clone(), storage.clone())
        .await
        .unwrap();
    app.commit(storage.clone()).await;
    insta::assert_json_snapshot!(app.app_hash.as_bytes());
}

// Note: this tests every action except for `Ics20Withdrawal` and `IbcRelay`.
//
// If new actions are added to the app, they must be added to this test,
// and the respective PR must be marked as breaking.
#[expect(clippy::too_many_lines, reason = "it's a test")]
#[tokio::test]
async fn app_execute_transaction_with_every_action_snapshot() {
    use astria_core::protocol::transaction::v1::action::{
        FeeAssetChange,
        InitBridgeAccount,
        SudoAddressChange,
    };

    let alice = get_alice_signing_key();
    let bridge = get_bridge_signing_key();
    let bridge_address = astria_address(&bridge.address_bytes());
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let carol_address = astria_address_from_hex_string(CAROL_ADDRESS);

    let accounts = {
        let mut acc = default_genesis_accounts();
        acc.push(Account {
            address: bridge_address,
            balance: 1_000_000_000,
        });
        acc.into_iter().map(Protobuf::into_raw).collect()
    };
    let genesis_state = astria_core::generated::protocol::genesis::v1::GenesisAppState {
        accounts,
        authority_sudo_address: Some(alice.try_address(ASTRIA_PREFIX).unwrap().to_raw()),
        ibc_sudo_address: Some(alice.try_address(ASTRIA_PREFIX).unwrap().to_raw()),
        ..proto_genesis_state()
    }
    .try_into()
    .unwrap();
    let (mut app, storage) = initialize_app_with_storage(Some(genesis_state), vec![]).await;

    // setup for ValidatorUpdate action
    let update = ValidatorUpdate {
        power: 100,
        verification_key: crate::benchmark_and_test_utils::verification_key(1),
    };

    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");

    let tx_bundleable_general = TransactionBody::builder()
        .actions(vec![
            Transfer {
                to: bob_address,
                amount: 333_333,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
            RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data: Bytes::from_static(b"hello world"),
                fee_asset: nria().into(),
            }
            .into(),
            Action::ValidatorUpdate(update.clone()),
        ])
        .chain_id("test")
        .try_build()
        .unwrap();

    let tx_bundleable_sudo = TransactionBody::builder()
        .actions(vec![
            IbcRelayerChange::Addition(bob_address).into(),
            IbcRelayerChange::Addition(carol_address).into(),
            IbcRelayerChange::Removal(bob_address).into(),
            FeeAssetChange::Addition("test-0".parse().unwrap()).into(),
            FeeAssetChange::Addition("test-1".parse().unwrap()).into(),
            FeeAssetChange::Removal("test-0".parse().unwrap()).into(),
        ])
        .nonce(1)
        .chain_id("test")
        .try_build()
        .unwrap();

    let tx_sudo_ibc = TransactionBody::builder()
        .actions(vec![
            IbcSudoChange {
                new_address: bob_address,
            }
            .into(),
        ])
        .nonce(2)
        .chain_id("test")
        .try_build()
        .unwrap();

    let tx_sudo = TransactionBody::builder()
        .actions(vec![
            SudoAddressChange {
                new_address: bob_address,
            }
            .into(),
        ])
        .nonce(3)
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx_general_bundleable = Arc::new(tx_bundleable_general.sign(&alice));
    app.execute_transaction(signed_tx_general_bundleable)
        .await
        .unwrap();

    let signed_tx_sudo_bundleable = Arc::new(tx_bundleable_sudo.sign(&alice));
    app.execute_transaction(signed_tx_sudo_bundleable)
        .await
        .unwrap();

    let signed_tx_sudo_ibc = Arc::new(tx_sudo_ibc.sign(&alice));
    app.execute_transaction(signed_tx_sudo_ibc).await.unwrap();

    let signed_tx_sudo = Arc::new(tx_sudo.sign(&alice));
    app.execute_transaction(signed_tx_sudo).await.unwrap();

    let tx = TransactionBody::builder()
        .actions(vec![
            InitBridgeAccount {
                rollup_id,
                asset: nria().into(),
                fee_asset: nria().into(),
                sudo_address: None,
                withdrawer_address: None,
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap();
    let signed_tx = Arc::new(tx.sign(&bridge));
    app.execute_transaction(signed_tx).await.unwrap();

    let tx_bridge_bundleable = TransactionBody::builder()
        .actions(vec![
            BridgeLock {
                to: bridge_address,
                amount: 100,
                asset: nria().into(),
                fee_asset: nria().into(),
                destination_chain_address: "nootwashere".to_string(),
            }
            .into(),
            BridgeUnlock {
                to: bob_address,
                amount: 10,
                fee_asset: nria().into(),
                memo: String::new(),
                bridge_address: astria_address(&bridge.address_bytes()),
                rollup_block_number: 1,
                rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
            }
            .into(),
        ])
        .nonce(1)
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx_bridge_bundleable.sign(&bridge));
    app.execute_transaction(signed_tx).await.unwrap();

    let tx_bridge = TransactionBody::builder()
        .actions(vec![
            BridgeSudoChange {
                bridge_address,
                new_sudo_address: Some(bob_address),
                new_withdrawer_address: Some(bob_address),
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .nonce(2)
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = Arc::new(tx_bridge.sign(&bridge));
    app.execute_transaction(signed_tx).await.unwrap();

    let sudo_address = app.state_delta.get_sudo_address().await.unwrap();
    app.end_block(1, &sudo_address).await.unwrap();

    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    insta::assert_json_snapshot!(app.app_hash.as_bytes());
}
