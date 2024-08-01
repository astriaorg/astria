use std::collections::HashMap;

use astria_core::{
    primitive::v1::{
        asset::TracePrefixed,
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
    sequencer::Account,
    sequencerblock::v1alpha1::block::Deposit,
};
use cnidarium::StateDelta;
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
    accounts::StateReadExt as _,
    app::test_utils::*,
    assets::StateReadExt as _,
    authority::{
        StateReadExt as _,
        StateWriteExt as _,
        ValidatorSet,
    },
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    proposal::commitment::generate_rollup_datas_commitment,
    state_ext::StateReadExt as _,
    test_utils::{
        astria_address,
        astria_address_from_hex_string,
        nria,
        verification_key,
    },
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
                .get_account_balance(address, nria())
                .await
                .unwrap(),
        );
    }

    assert_eq!(
        app.state.get_native_asset().await.unwrap(),
        "nria".parse::<TracePrefixed>().unwrap()
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
    use tendermint::abci::types;

    let initial_validator_set = vec![
        ValidatorUpdate {
            power: 100u32,
            verification_key: verification_key(1),
        },
        ValidatorUpdate {
            power: 1u32,
            verification_key: verification_key(2),
        },
    ];

    let mut app = initialize_app(None, initial_validator_set.clone()).await;

    let misbehavior = types::Misbehavior {
        kind: types::MisbehaviorKind::Unknown,
        validator: types::Validator {
            address: crate::test_utils::verification_key(1).address_bytes(),
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
    assert_eq!(validator_set.get(verification_key(2)).unwrap().power, 1,);
}

#[tokio::test]
async fn app_commit() {
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;
    assert_eq!(app.state.get_block_height().await.unwrap(), 0);

    for Account {
        address,
        balance,
    } in default_genesis_accounts()
    {
        assert_eq!(
            balance,
            app.state
                .get_account_balance(address, nria())
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
            snapshot.get_account_balance(address, nria()).await.unwrap(),
            balance
        );
    }
}

#[tokio::test]
async fn app_transfer_block_fees_to_sudo() {
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

    let alice = get_alice_signing_key();

    // transfer funds from Alice to Bob; use native token for fee payment
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let amount = 333_333;
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            TransferAction {
                to: bob_address,
                amount,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&alice);

    let proposer_address: tendermint::account::Id = [99u8; 20].to_vec().try_into().unwrap();

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
            .get_account_balance(astria_address_from_hex_string(JUDY_ADDRESS), nria())
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

    let alice = get_alice_signing_key();
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

    let bridge_address = astria_address(&[99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx.put_bridge_account_rollup_id(bridge_address, &rollup_id);
    state_tx
        .put_bridge_account_ibc_asset(bridge_address, nria())
        .unwrap();
    app.apply(state_tx);
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    let amount = 100;
    let lock_action = BridgeLockAction {
        to: bridge_address,
        amount,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
    };
    let sequence_action = SequenceAction {
        rollup_id,
        data: b"hello world".to_vec(),
        fee_asset: nria().into(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![lock_action.into(), sequence_action.into()],
    };

    let signed_tx = tx.into_signed(&alice);

    let expected_deposit = Deposit::new(
        bridge_address,
        rollup_id,
        amount,
        nria().into(),
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
    assert_eq!(*deposits[0], expected_deposit);
}

// it's a test, so allow a lot of lines
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn app_execution_results_match_proposal_vs_after_proposal() {
    let alice = get_alice_signing_key();
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

    let bridge_address = astria_address(&[99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let asset = nria().clone();

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx.put_bridge_account_rollup_id(bridge_address, &rollup_id);
    state_tx
        .put_bridge_account_ibc_asset(bridge_address, &asset)
        .unwrap();
    app.apply(state_tx);
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    let amount = 100;
    let lock_action = BridgeLockAction {
        to: bridge_address,
        amount,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
    };
    let sequence_action = SequenceAction {
        rollup_id,
        data: b"hello world".to_vec(),
        fee_asset: nria().into(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![lock_action.into(), sequence_action.into()],
    };

    let signed_tx = tx.into_signed(&alice);

    let expected_deposit = Deposit::new(
        bridge_address,
        rollup_id,
        amount,
        nria().into(),
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
    app.mempool.insert(signed_tx, 0).await.unwrap();

    let proposer_address = [88u8; 20].to_vec().try_into().unwrap();
    let prepare_proposal = PrepareProposal {
        height: 1u32.into(),
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address,
        txs: vec![],
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
    assert_eq!(app.mempool.len().await, 0);

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
    let finalize_block_after_process_proposal_result = app
        .finalize_block(finalize_block, storage.clone())
        .await
        .unwrap();

    assert_eq!(
        finalize_block_after_process_proposal_result.app_hash,
        finalize_block_result.app_hash
    );
}

#[tokio::test]
async fn app_prepare_proposal_cometbft_max_bytes_overflow_ok() {
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    // create txs which will cause cometBFT overflow
    let alice = get_alice_signing_key();
    let tx_pass = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from([1u8; 32]),
                data: vec![1u8; 100_000],
                fee_asset: nria().into(),
            }
            .into(),
        ],
    }
    .into_signed(&alice);
    let tx_overflow = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(1)
            .chain_id("test")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from([1u8; 32]),
                data: vec![1u8; 100_000],
                fee_asset: nria().into(),
            }
            .into(),
        ],
    }
    .into_signed(&alice);

    app.mempool.insert(tx_pass, 0).await.unwrap();
    app.mempool.insert(tx_overflow, 0).await.unwrap();

    // send to prepare_proposal
    let prepare_args = abci::request::PrepareProposal {
        max_tx_bytes: 200_000,
        txs: vec![],
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
        "total transaction length should be three, including the two commitments and the one tx \
         that fit"
    );
    assert_eq!(
        app.mempool.len().await,
        1,
        "mempool should have re-added the tx that was too large"
    );
}

#[tokio::test]
async fn app_prepare_proposal_sequencer_max_bytes_overflow_ok() {
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    // create txs which will cause sequencer overflow (max is currently 256_000 bytes)
    let alice = get_alice_signing_key();
    let tx_pass = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(0)
            .chain_id("test")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from([1u8; 32]),
                data: vec![1u8; 200_000],
                fee_asset: nria().into(),
            }
            .into(),
        ],
    }
    .into_signed(&alice);
    let tx_overflow = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(1)
            .chain_id("test")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from([1u8; 32]),
                data: vec![1u8; 100_000],
                fee_asset: nria().into(),
            }
            .into(),
        ],
    }
    .into_signed(&alice);

    app.mempool.insert(tx_pass, 0).await.unwrap();
    app.mempool.insert(tx_overflow, 0).await.unwrap();

    // send to prepare_proposal
    let prepare_args = abci::request::PrepareProposal {
        max_tx_bytes: 600_000, // make large enough to overflow sequencer bytes first
        txs: vec![],
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
        "total transaction length should be three, including the two commitments and the one tx \
         that fit"
    );
    assert_eq!(
        app.mempool.len().await,
        1,
        "mempool should have re-added the tx that was too large"
    );
}

#[tokio::test]
async fn app_end_block_validator_updates() {
    let initial_validator_set = vec![
        ValidatorUpdate {
            power: 100,
            verification_key: crate::test_utils::verification_key(1),
        },
        ValidatorUpdate {
            power: 1,
            verification_key: crate::test_utils::verification_key(2),
        },
    ];

    let mut app = initialize_app(None, initial_validator_set).await;
    let proposer_address = [0u8; 20];

    let validator_updates = vec![
        ValidatorUpdate {
            power: 0,
            verification_key: verification_key(0),
        },
        ValidatorUpdate {
            power: 100,
            verification_key: verification_key(1),
        },
        ValidatorUpdate {
            power: 100,
            verification_key: verification_key(2),
        },
    ];

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx
        .put_validator_updates(ValidatorSet::new_from_updates(validator_updates.clone()))
        .unwrap();
    app.apply(state_tx);

    let resp = app.end_block(1, proposer_address).await.unwrap();
    // we only assert length here as the ordering of the updates is not guaranteed
    // and validator::Update does not implement Ord
    assert_eq!(resp.validator_updates.len(), validator_updates.len());

    // validator with pubkey_a should be removed (power set to 0)
    // validator with pubkey_b should be updated
    // validator with pubkey_c should be added
    let validator_set = app.state.get_validator_set().await.unwrap();
    assert_eq!(validator_set.len(), 2);
    let validator_b = validator_set
        .get(verification_key(1).address_bytes())
        .unwrap();
    assert_eq!(validator_b.verification_key, verification_key(1));
    assert_eq!(validator_b.power, 100);
    let validator_c = validator_set
        .get(verification_key(2).address_bytes())
        .unwrap();
    assert_eq!(validator_c.verification_key, verification_key(2));
    assert_eq!(validator_c.power, 100);
    assert_eq!(app.state.get_validator_updates().await.unwrap().len(), 0);
}
