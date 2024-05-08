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
        default_genesis_accounts,
        get_alice_signing_key_and_address,
        initialize_app,
        initialize_app_with_storage,
        BOB_ADDRESS,
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

#[tokio::test]
async fn app_execute_transaction_transfer_snapshot() {
    let mut app = initialize_app(None, vec![]).await;

    let (alice_signing_key, _) = get_alice_signing_key_and_address();
    let bob_address = address_from_hex_string(BOB_ADDRESS);
    let value = 333_333;
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            TransferAction {
                to: bob_address,
                amount: value,
                asset_id: get_native_asset().id(),
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();
    insta::assert_json_snapshot!(app.app_hash.as_bytes());
}

#[tokio::test]
async fn app_execute_transaction_validator_update_snapshot() {
    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

    let genesis_state = GenesisState {
        accounts: default_genesis_accounts(),
        authority_sudo_address: alice_address,
        ibc_sudo_address: alice_address,
        ibc_relayer_addresses: vec![],
        native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
        ibc_params: IBCParameters::default(),
        allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
    };
    let mut app = initialize_app(Some(genesis_state), vec![]).await;

    let pub_key = tendermint::public_key::PublicKey::from_raw_ed25519(&[1u8; 32]).unwrap();
    let update = tendermint::validator::Update {
        pub_key,
        power: 100u32.into(),
    };

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![Action::ValidatorUpdate(update.clone())],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();
    insta::assert_json_snapshot!(app.app_hash.as_bytes());
}
