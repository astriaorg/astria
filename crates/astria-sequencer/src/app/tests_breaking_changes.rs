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
use std::collections::HashMap;

use astria_core::{
    primitive::v1::{
        asset::DEFAULT_NATIVE_ASSET_DENOM,
        Address,
        RollupId,
    },
    protocol::transaction::v1alpha1::{
        action::{
            BridgeLockAction,
            BridgeUnlockAction,
            IbcRelayerChangeAction,
            SequenceAction,
            TransferAction,
        },
        Action,
        TransactionParams,
        UnsignedTransaction,
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use cnidarium::StateDelta;
use penumbra_ibc::params::IBCParameters;
use prost::Message as _;
use tendermint::{
    abci,
    abci::types::CommitInfo,
    block::Round,
    Hash,
    Time,
};

use crate::{
    app::test_utils::{
        address_from_hex_string,
        default_fees,
        default_genesis_accounts,
        get_alice_signing_key_and_address,
        get_bridge_signing_key_and_address,
        initialize_app,
        initialize_app_with_storage,
        BOB_ADDRESS,
        CAROL_ADDRESS,
    },
    asset::get_native_asset,
    bridge::state_ext::StateWriteExt as _,
    genesis::GenesisState,
    proposal::commitment::generate_rollup_datas_commitment,
};

#[tokio::test]
async fn app_genesis_snapshot() {
    let app = initialize_app(None, vec![]).await;
    insta::assert_json_snapshot!(app.app_hash.as_bytes());
}

#[tokio::test]
async fn app_finalize_block_snapshot() {
    let (alice_signing_key, _) = get_alice_signing_key_and_address();
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

    let bridge_address = Address::from([99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let asset_id = get_native_asset().id();

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
    state_tx
        .put_bridge_account_asset_id(&bridge_address, &asset_id)
        .unwrap();
    app.apply(state_tx);

    // the state changes must be committed, as `finalize_block` will execute the
    // changes on the latest snapshot, not the app's `StateDelta`.
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    let amount = 100;
    let lock_action = BridgeLockAction {
        to: bridge_address,
        amount,
        asset_id,
        fee_asset_id: asset_id,
        destination_chain_address: "nootwashere".to_string(),
    };
    let sequence_action = SequenceAction {
        rollup_id,
        data: b"hello world".to_vec(),
        fee_asset_id: asset_id,
    };
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![lock_action.into(), sequence_action.into()],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);

    let expected_deposit = Deposit::new(
        bridge_address,
        rollup_id,
        amount,
        asset_id,
        "nootwashere".to_string(),
    );
    let deposits = HashMap::from_iter(vec![(rollup_id, vec![expected_deposit.clone()])]);
    let commitments = generate_rollup_datas_commitment(&[signed_tx.clone()], deposits.clone());

    let timestamp = Time::now();
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
    insta::assert_json_snapshot!(app.app_hash.as_bytes());
}

// Note: this tests every action except for `Ics20Withdrawal` and `IbcRelay`.
//
// If new actions are added to the app, they must be added to this test,
// and the respective PR must be marked as breaking.
#[allow(clippy::too_many_lines)]
#[tokio::test]
async fn app_execute_transaction_with_every_action_snapshot() {
    use astria_core::{
        primitive::v1::asset,
        protocol::transaction::v1alpha1::action::{
            FeeAssetChangeAction,
            InitBridgeAccountAction,
            SudoAddressChangeAction,
        },
    };

    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
    let (bridge_signing_key, bridge_address) = get_bridge_signing_key_and_address();
    let bob_address = address_from_hex_string(BOB_ADDRESS);
    let carol_address = address_from_hex_string(CAROL_ADDRESS);

    let genesis_state = GenesisState {
        accounts: default_genesis_accounts(),
        authority_sudo_address: alice_address,
        ibc_sudo_address: alice_address,
        ibc_relayer_addresses: vec![],
        native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
        ibc_params: IBCParameters::default(),
        allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        fees: default_fees(),
    };
    let (mut app, storage) = initialize_app_with_storage(Some(genesis_state), vec![]).await;

    // setup for ValidatorUpdate action
    let pub_key = tendermint::public_key::PublicKey::from_raw_ed25519(&[1u8; 32]).unwrap();
    let update = tendermint::validator::Update {
        pub_key,
        power: 100u32.into(),
    };

    // setup for BridgeLockAction
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let asset_id = get_native_asset().id();
    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
    state_tx
        .put_bridge_account_asset_id(&bridge_address, &asset_id)
        .unwrap();
    app.apply(state_tx);

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            TransferAction {
                to: bob_address,
                amount: 333_333,
                asset_id: get_native_asset().id(),
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data: b"hello world".to_vec(),
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
            Action::ValidatorUpdate(update.clone()),
            IbcRelayerChangeAction::Addition(bob_address).into(),
            IbcRelayerChangeAction::Addition(carol_address).into(),
            IbcRelayerChangeAction::Removal(bob_address).into(),
            // TODO: should fee assets be stored in state?
            FeeAssetChangeAction::Addition(asset::Id::from("test-0".to_string())).into(),
            FeeAssetChangeAction::Addition(asset::Id::from("test-1".to_string())).into(),
            FeeAssetChangeAction::Removal(asset::Id::from("test-0".to_string())).into(),
            InitBridgeAccountAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                asset_id: get_native_asset().id(),
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
            BridgeLockAction {
                to: bridge_address,
                amount: 100,
                asset_id: get_native_asset().id(),
                fee_asset_id: get_native_asset().id(),
                destination_chain_address: "nootwashere".to_string(),
            }
            .into(),
            SudoAddressChangeAction {
                new_address: bob_address,
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();

    // execute BridgeUnlock action
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            BridgeUnlockAction {
                to: bob_address,
                amount: 10,
                fee_asset_id: get_native_asset().id(),
                memo: vec![0u8; 32],
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&bridge_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();

    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    insta::assert_json_snapshot!(app.app_hash.as_bytes());
}
