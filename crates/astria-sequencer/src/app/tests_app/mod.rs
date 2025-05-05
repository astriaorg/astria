mod mempool;
mod upgrades;

use std::collections::HashMap;

use astria_core::{
    generated::{
        astria::sequencerblock::v1::RollupData as RawRollupData,
        price_feed::abci::v2::OracleVoteExtension as RawOracleVoteExtension,
    },
    oracles::price_feed::{
        oracle::v2::CurrencyPairState,
        types::v2::{
            CurrencyPair,
            CurrencyPairId,
            CurrencyPairNonce,
            Price,
        },
    },
    primitive::v1::{
        asset::TracePrefixed,
        RollupId,
        TransactionId,
    },
    protocol::{
        genesis::v1::Account,
        price_feed::v1::{
            CurrencyPairInfo,
            ExtendedCommitInfoWithCurrencyPairMapping,
        },
        transaction::v1::{
            action::{
                BridgeLock,
                RollupDataSubmission,
                SudoAddressChange,
                Transfer,
            },
            TransactionBody,
        },
    },
    sequencerblock::v1::block::{
        Deposit,
        RollupData,
    },
    upgrades::test_utils::UpgradesBuilder,
};
use benchmark_and_test_utils::{
    default_genesis_accounts,
    mock_balances,
    mock_tx_cost,
    BOB_ADDRESS,
    JUDY_ADDRESS,
};
use cnidarium::StateDelta;
use prost::{
    bytes::Bytes,
    Message as _,
};
use tendermint::{
    abci::{
        self,
        request::{
            PrepareProposal,
            ProcessProposal,
        },
        types::{
            BlockSignatureInfo,
            CommitInfo,
            ExtendedCommitInfo,
            ExtendedVoteInfo,
            Validator,
        },
    },
    account,
    block::{
        header::Version,
        BlockIdFlag,
        Header,
        Height,
        Round,
    },
    AppHash,
    Hash,
    Time,
};
use tendermint_proto::types::CanonicalVoteExtension;

use super::*;
use crate::{
    accounts::StateReadExt as _,
    app::{
        benchmark_and_test_utils::AppInitializer,
        test_utils::*,
    },
    assets::StateReadExt as _,
    authority::{
        StateReadExt as _,
        StateWriteExt as _,
        ValidatorSet,
    },
    benchmark_and_test_utils::{
        astria_address,
        astria_address_from_hex_string,
        nria,
        verification_key,
    },
    bridge::StateWriteExt as _,
    fees::StateReadExt as _,
    grpc::StateReadExt as _,
    oracles::price_feed::oracle::state_ext::StateWriteExt,
    proposal::commitment::generate_rollup_datas_commitment,
};

fn default_tendermint_header() -> Header {
    Header {
        app_hash: AppHash::try_from(vec![]).unwrap(),
        chain_id: "test".to_string().try_into().unwrap(),
        consensus_hash: Hash::default(),
        data_hash: Some(Hash::try_from([0u8; 32].to_vec()).unwrap()),
        evidence_hash: Some(Hash::default()),
        height: Height::from(2_u8),
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
    let (app, _storage) = AppInitializer::new().init().await;
    assert_eq!(app.state.get_block_height().await.unwrap(), 0);

    for Account {
        address,
        balance,
    } in default_genesis_accounts()
    {
        assert_eq!(
            balance,
            app.state
                .get_account_balance(&address, &nria())
                .await
                .unwrap(),
        );
    }

    assert_eq!(
        app.state.get_native_asset().await.unwrap(),
        Some("nria".parse::<TracePrefixed>().unwrap()),
    );
}

#[tokio::test]
async fn app_pre_execute_transactions() {
    let (mut app, _storage) = AppInitializer::new().init().await;

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
            name: "test1".parse().unwrap(),
            power: 100u32,
            verification_key: verification_key(1),
        },
        ValidatorUpdate {
            name: "test2".parse().unwrap(),
            power: 1u32,
            verification_key: verification_key(2),
        },
    ];

    let (mut app, _storage) = AppInitializer::new()
        .with_genesis_validators(initial_validator_set.clone())
        .with_upgrades(UpgradesBuilder::new().set_aspen(Some(100)).build())
        .init()
        .await;

    let misbehavior = types::Misbehavior {
        kind: types::MisbehaviorKind::Unknown,
        validator: types::Validator {
            address: *verification_key(1).address_bytes(),
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
    let validator_set = app.state.pre_aspen_get_validator_set().await.unwrap();
    assert_eq!(validator_set.len(), 1);
    assert_eq!(validator_set.get(&verification_key(2)).unwrap().power, 1,);
}

#[tokio::test]
async fn app_commit() {
    let (mut app, storage) = AppInitializer::new().init().await;
    assert_eq!(app.state.get_block_height().await.unwrap(), 0);

    for Account {
        address,
        balance,
    } in default_genesis_accounts()
    {
        assert_eq!(
            balance,
            app.state
                .get_account_balance(&address, &nria())
                .await
                .unwrap()
        );
    }

    // commit should write the changes to the underlying storage
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await.unwrap();

    let snapshot = storage.latest_snapshot();
    assert_eq!(snapshot.get_block_height().await.unwrap(), 0);

    for Account {
        address,
        balance,
    } in default_genesis_accounts()
    {
        assert_eq!(
            snapshot
                .get_account_balance(&address, &nria())
                .await
                .unwrap(),
            balance
        );
    }
}

#[tokio::test]
async fn app_transfer_block_fees_to_sudo() {
    let (mut app, storage) = AppInitializer::new().init().await;
    let height = run_until_aspen_applied(&mut app, storage.clone()).await;

    let alice = get_alice_signing_key();

    // transfer funds from Alice to Bob; use native token for fee payment
    let bob_address = astria_address_from_hex_string(BOB_ADDRESS);
    let amount = 333_333;
    let tx = TransactionBody::builder()
        .actions(vec![Transfer {
            to: bob_address,
            amount,
            asset: nria().into(),
            fee_asset: nria().into(),
        }
        .into()])
        .chain_id("test")
        .try_build()
        .unwrap();

    let signed_tx = tx.sign(&alice);

    let proposer_address: tendermint::account::Id = [99u8; 20].to_vec().try_into().unwrap();
    let finalize_block = abci::request::FinalizeBlock {
        hash: Hash::try_from([0u8; 32].to_vec()).unwrap(),
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address,
        txs: transactions_with_extended_commit_info_and_commitments(
            height,
            &[Arc::new(signed_tx)],
            None,
        ),
        decided_last_commit: CommitInfo {
            votes: vec![],
            round: Round::default(),
        },
        misbehavior: vec![],
    };
    app.finalize_block(finalize_block, storage.clone())
        .await
        .unwrap();
    app.commit(storage).await.unwrap();

    // assert that transaction fees were transferred to the block proposer
    let transfer_base_fee = app
        .state
        .get_fees::<Transfer>()
        .await
        .expect("should not error fetching transfer fees")
        .expect("transfer fees should be stored")
        .base();
    assert_eq!(
        app.state
            .get_account_balance(&astria_address_from_hex_string(JUDY_ADDRESS), &nria())
            .await
            .unwrap(),
        transfer_base_fee,
    );
    assert_eq!(app.state.get_block_fees().len(), 0);
}

#[tokio::test]
async fn app_create_sequencer_block_with_sequenced_data_and_deposits() {
    let alice = get_alice_signing_key();
    let (mut app, storage) = AppInitializer::new().init().await;
    let height = run_until_aspen_applied(&mut app, storage.clone()).await;

    let bridge_address = astria_address(&[99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let starting_index_of_action = 0;

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    state_tx
        .put_bridge_account_ibc_asset(&bridge_address, nria())
        .unwrap();
    // Put a deposit from a previous block to ensure it is not mixed in with deposits for this
    // block (it has a different amount and tx ID to the later deposit).
    let old_deposit = Deposit {
        bridge_address,
        rollup_id,
        amount: 99,
        asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
        source_transaction_id: TransactionId::new([99; 32]),
        source_action_index: starting_index_of_action,
    };
    state_tx
        .put_deposits(
            &[32u8; 32],
            HashMap::from_iter([(rollup_id, vec![old_deposit])]),
        )
        .unwrap();
    app.apply(state_tx);
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await.unwrap();

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

    let finalize_block = abci::request::FinalizeBlock {
        hash: Hash::try_from([0u8; 32].to_vec()).unwrap(),
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: transactions_with_extended_commit_info_and_commitments(
            height,
            &[Arc::new(signed_tx)],
            Some(deposits),
        ),
        decided_last_commit: CommitInfo {
            votes: vec![],
            round: Round::default(),
        },
        misbehavior: vec![],
    };
    app.finalize_block(finalize_block, storage.clone())
        .await
        .unwrap();
    app.commit(storage).await.unwrap();

    let block = app
        .state
        .get_sequencer_block_by_height(height.value())
        .await
        .unwrap();
    let mut deposits = vec![];
    for (_, rollup_data) in block.rollup_transactions() {
        for tx in rollup_data.transactions() {
            let rollup_data =
                RollupData::try_from_raw(RawRollupData::decode(tx.as_ref()).unwrap()).unwrap();
            if let RollupData::Deposit(deposit) = rollup_data {
                deposits.push(deposit);
            }
        }
    }
    assert_eq!(deposits.len(), 1);
    assert_eq!(*deposits[0], expected_deposit);
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "it's a test, so allow a lot of lines"
)]
async fn app_execution_results_match_proposal_vs_after_proposal() {
    let alice = get_alice_signing_key();
    let (mut app, storage) = AppInitializer::new().init().await;
    let height = run_until_aspen_applied(&mut app, storage.clone()).await;

    let bridge_address = astria_address(&[99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let asset = nria().clone();
    let starting_index_of_action = 0;

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    state_tx
        .put_bridge_account_ibc_asset(&bridge_address, &asset)
        .unwrap();
    app.apply(state_tx);
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await.unwrap();

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

    let signed_tx = Arc::new(tx.sign(&alice));

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

    let timestamp = Time::now();
    let block_hash = Hash::Sha256([99u8; 32]);
    let finalize_block = abci::request::FinalizeBlock {
        hash: block_hash,
        height,
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: transactions_with_extended_commit_info_and_commitments(
            height,
            &[signed_tx.clone()],
            Some(deposits),
        ),
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
    app.mempool
        .insert(signed_tx, 0, mock_balances(0, 0), mock_tx_cost(0, 0, 0))
        .await
        .unwrap();

    let proposer_address = [88u8; 20].to_vec().try_into().unwrap();
    let prepare_proposal = PrepareProposal {
        height,
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address,
        txs: vec![],
        max_tx_bytes: 1_000_000,
        local_last_commit: Some(ExtendedCommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
    };

    let prepare_proposal_result = app
        .prepare_proposal(prepare_proposal, storage.clone())
        .await
        .unwrap();
    assert_eq!(prepare_proposal_result.txs, finalize_block.txs);

    app.mempool.run_maintenance(&app.state, false).await;

    assert_eq!(app.mempool.len().await, 0);

    // call process_proposal - should not re-execute anything.
    let process_proposal = abci::request::ProcessProposal {
        hash: block_hash,
        height,
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address,
        txs: finalize_block.txs.clone(),
        proposed_last_commit: Some(CommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
    };

    app.process_proposal(process_proposal.clone(), storage.clone())
        .await
        .unwrap();

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
    let (mut app, storage) = AppInitializer::new().init().await;
    let height = run_until_aspen_applied(&mut app, storage.clone()).await;

    // create txs which will cause cometBFT overflow
    let alice = get_alice_signing_key();
    let tx_pass = TransactionBody::builder()
        .actions(vec![RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 100_000]),
            fee_asset: nria().into(),
        }
        .into()])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&alice);

    let tx_overflow = TransactionBody::builder()
        .actions(vec![RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 100_000]),
            fee_asset: nria().into(),
        }
        .into()])
        .chain_id("test")
        .nonce(1)
        .try_build()
        .unwrap()
        .sign(&alice);

    app.mempool
        .insert(
            Arc::new(tx_pass),
            0,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();
    app.mempool
        .insert(
            Arc::new(tx_overflow),
            0,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();

    // send to prepare_proposal
    let prepare_args = abci::request::PrepareProposal {
        max_tx_bytes: 200_000,
        txs: vec![],
        local_last_commit: Some(ExtendedCommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: account::Id::new([1u8; 20]),
    };

    let result = app
        .prepare_proposal(prepare_args, storage)
        .await
        .expect("too large transactions should not cause prepare proposal to fail");

    // run maintence to clear out transactions
    app.mempool.run_maintenance(&app.state, false).await;

    // see only first tx made it in
    assert_eq!(
        result.txs.len(),
        4,
        "total transaction length should be four, including the extended commit info, two \
         commitments and the one tx that fit"
    );
    assert_eq!(
        app.mempool.len().await,
        1,
        "mempool should have re-added the tx that was too large"
    );
}

#[tokio::test]
async fn app_prepare_proposal_sequencer_max_bytes_overflow_ok() {
    let (mut app, storage) = AppInitializer::new().init().await;
    let height = run_until_aspen_applied(&mut app, storage.clone()).await;

    // create txs which will cause sequencer overflow (max is currently 256_000 bytes)
    let alice = get_alice_signing_key();
    let tx_pass = TransactionBody::builder()
        .actions(vec![RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 200_000]),
            fee_asset: nria().into(),
        }
        .into()])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&alice);
    let tx_overflow = TransactionBody::builder()
        .actions(vec![RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 100_000]),
            fee_asset: nria().into(),
        }
        .into()])
        .nonce(1)
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&alice);

    app.mempool
        .insert(
            Arc::new(tx_pass),
            0,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();
    app.mempool
        .insert(
            Arc::new(tx_overflow),
            0,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();

    // send to prepare_proposal
    let prepare_args = abci::request::PrepareProposal {
        max_tx_bytes: 600_000, // make large enough to overflow sequencer bytes first
        txs: vec![],
        local_last_commit: Some(ExtendedCommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: account::Id::new([1u8; 20]),
    };

    let result = app
        .prepare_proposal(prepare_args, storage)
        .await
        .expect("too large transactions should not cause prepare proposal to fail");

    // run maintence to clear out transactions
    app.mempool.run_maintenance(&app.state, false).await;

    // see only first tx made it in
    assert_eq!(
        result.txs.len(),
        4,
        "total transaction length should be four, including the extended commit info, two \
         commitments and the one tx that fit"
    );
    assert_eq!(
        app.mempool.len().await,
        1,
        "mempool should have re-added the tx that was too large"
    );
}

#[tokio::test]
async fn app_process_proposal_sequencer_max_bytes_overflow_fail() {
    let (mut app, storage) = AppInitializer::new().init().await;
    let height = run_until_aspen_applied(&mut app, storage.clone()).await;

    // create txs which will cause sequencer overflow (max is currently 256_000 bytes)
    let alice = get_alice_signing_key();
    let tx_pass = TransactionBody::builder()
        .actions(vec![RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 200_000]),
            fee_asset: nria().into(),
        }
        .into()])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&alice);
    let tx_overflow = TransactionBody::builder()
        .actions(vec![RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 100_000]),
            fee_asset: nria().into(),
        }
        .into()])
        .nonce(1)
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&alice);

    let txs = vec![Arc::new(tx_pass), Arc::new(tx_overflow)];

    let process_proposal = ProcessProposal {
        hash: Hash::default(),
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: transactions_with_extended_commit_info_and_commitments(height, &txs, None),
        proposed_last_commit: Some(CommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
    };

    let result = app
        .process_proposal(process_proposal.clone(), storage.clone())
        .await
        .expect_err("expected max sequenced data limit error");

    assert!(
        format!("{result:?}").contains("max block sequenced data limit passed"),
        "process proposal should fail due to max sequenced data limit"
    );
}

#[tokio::test]
async fn app_process_proposal_transaction_fails_to_execute_fails() {
    let (mut app, storage) = AppInitializer::new().init().await;
    let height = run_until_aspen_applied(&mut app, storage.clone()).await;

    // create txs which will cause transaction execution failure
    let alice = get_alice_signing_key();
    let tx_fail = TransactionBody::builder()
        .actions(vec![SudoAddressChange {
            new_address: astria_address_from_hex_string(BOB_ADDRESS),
        }
        .into()])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&alice);

    let txs = vec![Arc::new(tx_fail)];

    let process_proposal = ProcessProposal {
        hash: Hash::default(),
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: transactions_with_extended_commit_info_and_commitments(height, &txs, None),
        proposed_last_commit: Some(CommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
    };

    let result = app
        .process_proposal(process_proposal.clone(), storage.clone())
        .await
        .expect_err("expected transaction execution failure");

    assert!(
        format!("{result:?}").contains("transaction failed to execute"),
        "process proposal should fail due transaction execution failure"
    );
}

#[tokio::test]
async fn app_end_block_validator_updates() {
    let initial_validator_set = vec![
        ValidatorUpdate {
            name: "test1".parse().unwrap(),
            power: 100,
            verification_key: verification_key(1),
        },
        ValidatorUpdate {
            name: "test2".parse().unwrap(),
            power: 1,
            verification_key: verification_key(2),
        },
    ];

    let (mut app, _storage) = AppInitializer::new()
        .with_genesis_validators(initial_validator_set)
        .init()
        .await;
    let proposer_address = [0u8; 20];

    let validator_updates = vec![
        ValidatorUpdate {
            name: "test0".parse().unwrap(),
            power: 0,
            verification_key: verification_key(0),
        },
        ValidatorUpdate {
            name: "test1".parse().unwrap(),
            power: 100,
            verification_key: verification_key(1),
        },
        ValidatorUpdate {
            name: "test2".parse().unwrap(),
            power: 100,
            verification_key: verification_key(2),
        },
    ];

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx
        .put_block_validator_updates(ValidatorSet::new_from_updates(validator_updates.clone()))
        .unwrap();
    app.apply(state_tx);

    let resp = app.end_block(1, &proposer_address).await.unwrap();
    // we only assert length here as the ordering of the updates is not guaranteed
    // and validator::Update does not implement Ord
    assert_eq!(resp.validator_updates.len(), validator_updates.len());

    // validator with pubkey_a should be removed (power set to 0)
    // validator with pubkey_b should be updated
    // validator with pubkey_c should be added
    let validator_set = app.state.pre_aspen_get_validator_set().await.unwrap();
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
    assert_eq!(
        app.state.get_block_validator_updates().await.unwrap().len(),
        0
    );
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "it's a test, so allow a lot of lines"
)]
async fn app_proposal_fingerprint_triggers_update() {
    let alice = get_alice_signing_key();
    let (mut app, storage) = AppInitializer::new().init().await;
    let height = run_until_aspen_applied(&mut app, storage.clone()).await;

    let bridge_address = astria_address(&[99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let asset = nria().clone();
    let starting_index_of_action = 0;

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    state_tx
        .put_bridge_account_ibc_asset(&bridge_address, &asset)
        .unwrap();
    app.apply(state_tx);
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await.unwrap();

    // Commit should clear the fingerprint
    assert_eq!(*app.execution_state.data(), ExecutionState::Unset);

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

    let signed_tx = Arc::new(tx.sign(&alice));

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

    let timestamp = Time::now();
    let raw_hash = [99u8; 32];
    let block_hash = Hash::Sha256(raw_hash);
    let txs = vec![signed_tx.to_raw().encode_to_vec().into()];
    let txs_with_commitments = transactions_with_extended_commit_info_and_commitments(
        height,
        &[signed_tx.clone()],
        Some(deposits.clone()),
    );

    // These two proposals match exactly, except for the proposer
    let prepare_proposal = PrepareProposal {
        max_tx_bytes: 1_000_000,
        height,
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: txs.clone(),
        local_last_commit: Some(ExtendedCommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
    };
    let match_process_proposal = ProcessProposal {
        hash: block_hash,
        height,
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: txs_with_commitments.clone(),
        proposed_last_commit: Some(CommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
    };
    let non_match_process_proposal = ProcessProposal {
        hash: block_hash,
        height,
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address: [1u8; 20].to_vec().try_into().unwrap(),
        txs: txs_with_commitments.clone(),
        proposed_last_commit: Some(CommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
    };
    let finalize_block = abci::request::FinalizeBlock {
        hash: block_hash,
        height,
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: txs_with_commitments.clone(),
        decided_last_commit: CommitInfo {
            votes: vec![],
            round: 0u16.into(),
        },
        misbehavior: vec![],
    };

    // Call PrepareProposal, then ProcessProposal where the proposals match
    // - fingerprint created during prepare_proposal
    // - after process_proposal, fingerprint should be updated to include the block hash indicating
    //   that the proposal execution data was utilized.
    // - the finalize block fingerprint should match
    app.mempool
        .insert(
            signed_tx.clone(),
            0,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();
    app.prepare_proposal(prepare_proposal.clone(), storage.clone())
        .await
        .unwrap();
    let ExecutionState::Prepared(cached_proposal) = app.execution_state.data().clone() else {
        panic!("should be in prepared state")
    };
    app.process_proposal(match_process_proposal.clone(), storage.clone())
        .await
        .unwrap();
    let expected_state = ExecutionState::ExecutedBlock {
        cached_block_hash: raw_hash,
        cached_proposal: Some(cached_proposal),
    };
    assert_eq!(*app.execution_state.data(), expected_state);
    app.finalize_block(finalize_block.clone(), storage.clone())
        .await
        .unwrap();
    assert_eq!(*app.execution_state.data(), expected_state);

    // Call PrepareProposal, then ProcessProposal where the proposals do not match
    // - validate the prepared proposal fingerprint
    // - after process proposal should have a ExecuteBlock fingerprint w/ no prepare fingerprint
    //   data
    // - finalize block should not change the fingerprint
    app.prepare_proposal(prepare_proposal.clone(), storage.clone())
        .await
        .unwrap();
    app.process_proposal(non_match_process_proposal.clone(), storage.clone())
        .await
        .unwrap();
    let expected_state = ExecutionState::ExecutedBlock {
        cached_block_hash: raw_hash,
        cached_proposal: None,
    };
    assert_eq!(*app.execution_state.data(), expected_state);
    app.finalize_block(finalize_block.clone(), storage.clone())
        .await
        .unwrap();
    assert_eq!(*app.execution_state.data(), expected_state);

    // Cannot use fingerprint to jump from prepare proposal straight to finalize block
    // - the fingerprint at the end of finalize block should not have the prepare proposal data
    app.prepare_proposal(prepare_proposal.clone(), storage.clone())
        .await
        .unwrap();
    app.finalize_block(finalize_block.clone(), storage.clone())
        .await
        .unwrap();
    assert_eq!(*app.execution_state.data(), expected_state);

    // Calling update state for new round should reset key
    app.update_state_for_new_round(&storage);
    assert_eq!(*app.execution_state.data(), ExecutionState::Unset);
}

#[expect(
    clippy::too_many_lines,
    reason = "it's a test, so allow a lot of lines"
)]
#[tokio::test]
async fn app_oracle_price_update_events_in_finalize_block() {
    let alice_signing_key = get_alice_signing_key();
    let initial_validator_set = vec![ValidatorUpdate {
        power: 100,
        verification_key: alice_signing_key.verification_key(),
        name: "test".parse().unwrap(),
    }];
    let (mut app, storage) = AppInitializer::new()
        .with_genesis_validators(initial_validator_set)
        .init()
        .await;
    let height = run_until_aspen_applied(&mut app, storage.clone()).await;

    let currency_pair: CurrencyPair = "ETH/USD".parse().unwrap();
    let id = CurrencyPairId::new(0);
    let currency_pair_state = CurrencyPairState {
        price: None,
        nonce: CurrencyPairNonce::new(0),
        id,
    };
    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx
        .put_currency_pair_state(currency_pair.clone(), currency_pair_state)
        .unwrap();
    app.apply(state_tx);
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await.unwrap();

    let mut prices = std::collections::BTreeMap::new();
    let price = Price::new(10000i128);
    let price_bytes = price.get().to_be_bytes().to_vec();
    let id_to_currency_pair = indexmap::indexmap! {
        id => CurrencyPairInfo{
            currency_pair: currency_pair.clone(),
            decimals: 0,
        }
    };
    let _ = prices.insert(id.get(), price_bytes.into());
    let extension_bytes = RawOracleVoteExtension {
        prices,
    }
    .encode_to_vec();
    let message_to_sign = CanonicalVoteExtension {
        extension: extension_bytes.clone(),
        height: i64::try_from(height.value()).unwrap(),
        round: 1,
        chain_id: "test".to_string(),
    }
    .encode_length_delimited_to_vec();

    let vote = ExtendedVoteInfo {
        validator: Validator {
            address: *alice_signing_key.verification_key().address_bytes(),
            power: 100u32.into(),
        },
        sig_info: BlockSignatureInfo::Flag(BlockIdFlag::Commit),
        vote_extension: extension_bytes.into(),
        extension_signature: Some(
            alice_signing_key
                .sign(&message_to_sign)
                .to_bytes()
                .to_vec()
                .try_into()
                .unwrap(),
        ),
    };
    let extended_commit_info = ExtendedCommitInfo {
        round: 1u16.into(),
        votes: vec![vote],
    };
    let extended_commit_info = ExtendedCommitInfoWithCurrencyPairMapping {
        extended_commit_info,
        id_to_currency_pair,
    };
    let encoded_extended_commit_info =
        DataItem::ExtendedCommitInfo(extended_commit_info.into_raw().encode_to_vec().into())
            .encode();
    let commitments = generate_rollup_datas_commitment::<true>(&[], HashMap::new());
    let txs_with_commit_info: Vec<Bytes> = commitments
        .into_iter()
        .chain(std::iter::once(encoded_extended_commit_info))
        .collect();

    let proposer_address: account::Id = [99u8; 20].to_vec().try_into().unwrap();
    let finalize_block = abci::request::FinalizeBlock {
        hash: Hash::try_from([0u8; 32].to_vec()).unwrap(),
        height: height.increment(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address,
        txs: txs_with_commit_info,
        decided_last_commit: CommitInfo {
            votes: vec![],
            round: Round::default(),
        },
        misbehavior: vec![],
    };
    let finalize_block = app
        .finalize_block(finalize_block, storage.clone())
        .await
        .unwrap();
    assert_eq!(finalize_block.events.len(), 1);

    let expected_event = Event::new(
        "price_update",
        [
            ("currency_pair", currency_pair.to_string()),
            ("price", price.to_string()),
        ],
    );
    assert_eq!(finalize_block.events[0], expected_event);
}
