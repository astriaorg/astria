use std::{
    collections::HashMap,
    sync::Arc,
};

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
        TransactionParams,
        UnsignedTransaction,
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use cnidarium::StateDelta;
use penumbra_ibc::params::IBCParameters;
use prost::Message as _;
use tendermint::{
    abci::{
        self,
        request::PrepareProposal,
        types::CommitInfo,
    },
    account,
    block::{
        header::Version,
        Header,
        Height,
        Round,
    },
    AppHash,
    Hash,
    Time,
};

use super::*;
use crate::{
    accounts::state_ext::StateReadExt as _,
    app::test_utils::*,
    asset::get_native_asset,
    authority::state_ext::StateReadExt as _,
    bridge::state_ext::{
        StateReadExt as _,
        StateWriteExt,
    },
    genesis::{
        Account,
        GenesisState,
    },
    proposal::commitment::generate_rollup_datas_commitment,
    state_ext::StateReadExt as _,
};

fn default_tendermint_header() -> Header {
    Header {
        app_hash: AppHash::try_from(vec![]).unwrap(),
        chain_id: "test".to_string().try_into().unwrap(),
        consensus_hash: Hash::default(),
        data_hash: Some(Hash::try_from([0u8; 32].to_vec()).unwrap()),
        evidence_hash: Some(Hash::default()),
        height: Height::default(),
        last_block_id: None,
        last_commit_hash: Some(Hash::default()),
        last_results_hash: Some(Hash::default()),
        next_validators_hash: Hash::default(),
        proposer_address: account::Id::try_from([0u8; 20].to_vec()).unwrap(),
        time: Time::now(),
        validators_hash: Hash::default(),
        version: Version {
            app: 0,
            block: 0,
        },
    }
}

#[tokio::test]
async fn app_genesis_and_init_chain() {
    let app = initialize_app(None, vec![]).await;
    assert_eq!(app.state.get_block_height().await.unwrap(), 0);

    for Account {
        address,
        balance,
    } in default_genesis_accounts()
    {
        assert_eq!(
            balance,
            app.state
                .get_account_balance(address, get_native_asset().id())
                .await
                .unwrap(),
        );
    }

    assert_eq!(
        app.state.get_native_asset_denom().await.unwrap(),
        DEFAULT_NATIVE_ASSET_DENOM
    );
}

#[tokio::test]
async fn app_pre_execute_transactions() {
    let mut app = initialize_app(None, vec![]).await;

    let block_data = BlockData {
        misbehavior: vec![],
        height: 1u8.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: account::Id::try_from([0u8; 20].to_vec()).unwrap(),
    };

    app.pre_execute_transactions(block_data.clone())
        .await
        .unwrap();
    assert_eq!(app.state.get_block_height().await.unwrap(), 1);
    assert_eq!(
        app.state.get_block_timestamp().await.unwrap(),
        block_data.time
    );
}

#[tokio::test]
async fn app_begin_block_remove_byzantine_validators() {
    use tendermint::{
        abci::types,
        validator,
    };

    let pubkey_a = tendermint::public_key::PublicKey::from_raw_ed25519(&[1; 32]).unwrap();
    let pubkey_b = tendermint::public_key::PublicKey::from_raw_ed25519(&[2; 32]).unwrap();

    let initial_validator_set = vec![
        validator::Update {
            pub_key: pubkey_a,
            power: 100u32.into(),
        },
        validator::Update {
            pub_key: pubkey_b,
            power: 1u32.into(),
        },
    ];

    let mut app = initialize_app(None, initial_validator_set.clone()).await;

    let misbehavior = types::Misbehavior {
        kind: types::MisbehaviorKind::Unknown,
        validator: types::Validator {
            address: tendermint::account::Id::from(pubkey_a)
                .as_bytes()
                .try_into()
                .unwrap(),
            power: 0u32.into(),
        },
        height: Height::default(),
        time: Time::now(),
        total_voting_power: 101u32.into(),
    };

    let mut begin_block = abci::request::BeginBlock {
        header: default_tendermint_header(),
        hash: Hash::default(),
        last_commit_info: CommitInfo {
            votes: vec![],
            round: Round::default(),
        },
        byzantine_validators: vec![misbehavior],
    };
    begin_block.header.height = 1u8.into();

    app.begin_block(&begin_block).await.unwrap();

    // assert that validator with pubkey_a is removed
    let validator_set = app.state.get_validator_set().await.unwrap();
    assert_eq!(validator_set.len(), 1);
    assert_eq!(
        validator_set.get(&pubkey_b.into()).unwrap().power,
        1u32.into()
    );
}

#[tokio::test]
async fn app_commit() {
    let genesis_state = GenesisState {
        accounts: default_genesis_accounts(),
        authority_sudo_address: Address::from([0; 20]),
        ibc_sudo_address: Address::from([0; 20]),
        ibc_relayer_addresses: vec![],
        native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
        ibc_params: IBCParameters::default(),
        allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
    };

    let (mut app, storage) = initialize_app_with_storage(Some(genesis_state), vec![]).await;
    assert_eq!(app.state.get_block_height().await.unwrap(), 0);

    let native_asset = get_native_asset().id();
    for Account {
        address,
        balance,
    } in default_genesis_accounts()
    {
        assert_eq!(
            balance,
            app.state
                .get_account_balance(address, native_asset)
                .await
                .unwrap()
        );
    }

    // commit should write the changes to the underlying storage
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    let snapshot = storage.latest_snapshot();
    assert_eq!(snapshot.get_block_height().await.unwrap(), 0);

    for Account {
        address,
        balance,
    } in default_genesis_accounts()
    {
        assert_eq!(
            snapshot
                .get_account_balance(address, native_asset)
                .await
                .unwrap(),
            balance
        );
    }
}

#[tokio::test]
async fn app_transfer_block_fees_to_proposer() {
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

    let (alice_signing_key, _) = get_alice_signing_key_and_address();
    let native_asset = get_native_asset().id();

    // transfer funds from Alice to Bob; use native token for fee payment
    let bob_address = address_from_hex_string(BOB_ADDRESS);
    let amount = 333_333;
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            TransferAction {
                to: bob_address,
                amount,
                asset_id: native_asset,
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);

    let proposer_address: tendermint::account::Id = [99u8; 20].to_vec().try_into().unwrap();
    let sequencer_proposer_address = Address::try_from_slice(proposer_address.as_bytes()).unwrap();

    let commitments = generate_rollup_datas_commitment(&[signed_tx.clone()], HashMap::new());

    let finalize_block = abci::request::FinalizeBlock {
        hash: Hash::try_from([0u8; 32].to_vec()).unwrap(),
        height: 1u32.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address,
        txs: commitments.into_transactions(vec![signed_tx.to_raw().encode_to_vec().into()]),
        decided_last_commit: CommitInfo {
            votes: vec![],
            round: Round::default(),
        },
        misbehavior: vec![],
    };
    app.finalize_block(finalize_block, storage.clone())
        .await
        .unwrap();
    app.commit(storage).await;

    // assert that transaction fees were transferred to the block proposer
    let transfer_fee = app.state.get_transfer_base_fee().await.unwrap();
    assert_eq!(
        app.state
            .get_account_balance(sequencer_proposer_address, native_asset)
            .await
            .unwrap(),
        transfer_fee,
    );
    assert_eq!(app.state.get_block_fees().await.unwrap().len(), 0);
}

#[tokio::test]
async fn app_create_sequencer_block_with_sequenced_data_and_deposits() {
    use astria_core::{
        generated::sequencerblock::v1alpha1::RollupData as RawRollupData,
        sequencerblock::v1alpha1::block::RollupData,
    };

    use crate::api_state_ext::StateReadExt as _;

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

    let finalize_block = abci::request::FinalizeBlock {
        hash: Hash::try_from([0u8; 32].to_vec()).unwrap(),
        height: 1u32.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: commitments.into_transactions(vec![signed_tx.to_raw().encode_to_vec().into()]),
        decided_last_commit: CommitInfo {
            votes: vec![],
            round: Round::default(),
        },
        misbehavior: vec![],
    };
    app.finalize_block(finalize_block, storage.clone())
        .await
        .unwrap();
    app.commit(storage).await;

    // ensure deposits are cleared at the end of the block
    let deposit_events = app.state.get_deposit_events(&rollup_id).await.unwrap();
    assert_eq!(deposit_events.len(), 0);

    let block = app.state.get_sequencer_block_by_height(1).await.unwrap();
    let mut deposits = vec![];
    for (_, rollup_data) in block.rollup_transactions() {
        for tx in rollup_data.transactions() {
            let rollup_data =
                RollupData::try_from_raw(RawRollupData::decode(tx.as_slice()).unwrap()).unwrap();
            if let RollupData::Deposit(deposit) = rollup_data {
                deposits.push(deposit);
            }
        }
    }
    assert_eq!(deposits.len(), 1);
    assert_eq!(deposits[0], expected_deposit);
}

// it's a test, so allow a lot of lines
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn app_execution_results_match_proposal_vs_after_proposal() {
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

    // call finalize_block with the given block data, which simulates executing a block
    // as a full node (non-validator node).
    let finalize_block_result = app
        .finalize_block(finalize_block.clone(), storage.clone())
        .await
        .unwrap();

    // don't commit the result, now call prepare_proposal with the same data.
    // this will reset the app state.
    // this simulates executing the same block as a validator (specifically the proposer).
    let proposer_address = [88u8; 20].to_vec().try_into().unwrap();
    let prepare_proposal = PrepareProposal {
        height: 1u32.into(),
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address,
        txs: vec![signed_tx.to_raw().encode_to_vec().into()],
        max_tx_bytes: 1_000_000,
        local_last_commit: None,
        misbehavior: vec![],
    };

    let prepare_proposal_result = app
        .prepare_proposal(prepare_proposal, storage.clone())
        .await
        .unwrap();
    assert_eq!(prepare_proposal_result.txs, finalize_block.txs);
    assert_eq!(app.executed_proposal_hash, Hash::default());
    assert_eq!(app.validator_address.unwrap(), proposer_address);

    // call process_proposal - should not re-execute anything.
    let process_proposal = abci::request::ProcessProposal {
        hash: block_hash,
        height: 1u32.into(),
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: finalize_block.txs.clone(),
        proposed_last_commit: None,
        misbehavior: vec![],
    };

    app.process_proposal(process_proposal.clone(), storage.clone())
        .await
        .unwrap();
    assert_eq!(app.executed_proposal_hash, block_hash);
    assert!(app.validator_address.is_none());

    let finalize_block_after_prepare_proposal_result = app
        .finalize_block(finalize_block.clone(), storage.clone())
        .await
        .unwrap();

    assert_eq!(
        finalize_block_after_prepare_proposal_result.app_hash,
        finalize_block_result.app_hash
    );

    // reset the app state and call process_proposal - should execute the block.
    // this simulates executing the block as a non-proposer validator.
    app.update_state_for_new_round(&storage);
    app.process_proposal(process_proposal, storage.clone())
        .await
        .unwrap();
    assert_eq!(app.executed_proposal_hash, block_hash);
    assert!(app.validator_address.is_none());
    let finalize_block_after_prepare_proposal_result = app
        .finalize_block(finalize_block, storage.clone())
        .await
        .unwrap();

    assert_eq!(
        finalize_block_after_prepare_proposal_result.app_hash,
        finalize_block_result.app_hash
    );
}

#[tokio::test]
async fn app_prepare_proposal_cometbft_max_bytes_overflow_ok() {
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

    // update storage with initalized genesis app state
    let intermediate_state = StateDelta::new(storage.latest_snapshot());
    let state = Arc::try_unwrap(std::mem::replace(
        &mut app.state,
        Arc::new(intermediate_state),
    ))
    .expect("we have exclusive ownership of the State at commit()");
    storage
        .commit(state)
        .await
        .expect("applying genesis state should be okay");

    // create txs which will cause cometBFT overflow
    let (alice_signing_key, _) = get_alice_signing_key_and_address();
    let tx_pass = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from([1u8; 32]),
                data: vec![1u8; 100_000],
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    }
    .into_signed(&alice_signing_key);
    let tx_overflow = UnsignedTransaction {
        params: TransactionParams {
            nonce: 1,
            chain_id: "test".to_string(),
        },
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from([1u8; 32]),
                data: vec![1u8; 100_000],
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    }
    .into_signed(&alice_signing_key);

    let txs: Vec<bytes::Bytes> = vec![
        tx_pass.to_raw().encode_to_vec().into(),
        tx_overflow.to_raw().encode_to_vec().into(),
    ];

    // send to prepare_proposal
    let prepare_args = abci::request::PrepareProposal {
        max_tx_bytes: 200_000,
        txs,
        local_last_commit: None,
        misbehavior: vec![],
        height: Height::default(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: account::Id::new([1u8; 20]),
    };

    let result = app
        .prepare_proposal(prepare_args, storage)
        .await
        .expect("too large transactions should not cause prepare proposal to fail");

    // see only first tx made it in
    assert_eq!(
        result.txs.len(),
        3,
        "total transaciton length should be three, including the two commitments and the one tx \
         that fit"
    );
}

#[tokio::test]
async fn app_prepare_proposal_sequencer_max_bytes_overflow_ok() {
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

    // update storage with initalized genesis app state
    let intermediate_state = StateDelta::new(storage.latest_snapshot());
    let state = Arc::try_unwrap(std::mem::replace(
        &mut app.state,
        Arc::new(intermediate_state),
    ))
    .expect("we have exclusive ownership of the State at commit()");
    storage
        .commit(state)
        .await
        .expect("applying genesis state should be okay");

    // create txs which will cause sequencer overflow (max is currently 256_000 bytes)
    let (alice_signing_key, _) = get_alice_signing_key_and_address();
    let tx_pass = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from([1u8; 32]),
                data: vec![1u8; 200_000],
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    }
    .into_signed(&alice_signing_key);
    let tx_overflow = UnsignedTransaction {
        params: TransactionParams {
            nonce: 1,
            chain_id: "test".to_string(),
        },
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from([1u8; 32]),
                data: vec![1u8; 100_000],
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    }
    .into_signed(&alice_signing_key);

    let txs: Vec<bytes::Bytes> = vec![
        tx_pass.to_raw().encode_to_vec().into(),
        tx_overflow.to_raw().encode_to_vec().into(),
    ];

    // send to prepare_proposal
    let prepare_args = abci::request::PrepareProposal {
        max_tx_bytes: 600_000, // make large enough to overflow sequencer bytes first
        txs,
        local_last_commit: None,
        misbehavior: vec![],
        height: Height::default(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: account::Id::new([1u8; 20]),
    };

    let result = app
        .prepare_proposal(prepare_args, storage)
        .await
        .expect("too large transactions should not cause prepare proposal to fail");

    // see only first tx made it in
    assert_eq!(
        result.txs.len(),
        3,
        "total transaciton length should be three, including the two commitments and the one tx \
         that fit"
    );
}
