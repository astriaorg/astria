use std::collections::HashMap;

use astria_core::{
    protocol::{
        fees::v1::TransferFeeComponents,
        genesis::v1::Account,
        transaction::v1::{
            action::{
                FeeChange,
                Transfer,
            },
            TransactionBody,
        },
    },
    Protobuf,
};
use benchmark_and_test_utils::{
    proto_genesis_state,
    ALICE_ADDRESS,
    CAROL_ADDRESS,
};
use prost::Message as _;
use tendermint::{
    abci::{
        self,
        types::CommitInfo,
    },
    account,
    block::{
        Height,
        Round,
    },
    Hash,
    Time,
};

use super::*;
use crate::{
    accounts::StateReadExt as _,
    app::test_utils::*,
    benchmark_and_test_utils::{
        astria_address_from_hex_string,
        nria,
    },
    proposal::commitment::generate_rollup_datas_commitment,
};

#[tokio::test]
async fn trigger_cleaning() {
    // check that cleaning is triggered by the prepare, process, and finalize block flows
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    // create tx which will cause mempool cleaning flag to be set
    let tx_trigger = TransactionBody::builder()
        .actions(vec![
            FeeChange::Transfer(TransferFeeComponents {
                base: 10,
                multiplier: 0,
            })
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&get_judy_signing_key());

    app.mempool
        .insert(
            Arc::new(tx_trigger.clone()),
            0,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();

    assert!(!app.recost_mempool, "flag should start out false");

    // trigger with prepare_proposal
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

    app.prepare_proposal(prepare_args, storage.clone())
        .await
        .expect("fee change with correct signer should pass prepare proposal");
    assert!(app.recost_mempool, "flag should have been set");

    // manually reset to trigger again
    app.recost_mempool = false;
    assert!(!app.recost_mempool, "flag should start out false");

    // trigger with process_proposal
    let commitments = generate_rollup_datas_commitment(&[tx_trigger.clone()], HashMap::new());
    let process_proposal = abci::request::ProcessProposal {
        hash: Hash::try_from([99u8; 32].to_vec()).unwrap(),
        height: 1u32.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: commitments.into_transactions(vec![tx_trigger.to_raw().encode_to_vec().into()]),
        proposed_last_commit: None,
        misbehavior: vec![],
    };

    app.process_proposal(process_proposal.clone(), storage.clone())
        .await
        .unwrap();
    assert!(app.recost_mempool, "flag should have been set");

    // trigger with finalize block
    app.recost_mempool = false;
    assert!(!app.recost_mempool, "flag should start out false");
    let commitments = generate_rollup_datas_commitment(&[tx_trigger.clone()], HashMap::new());
    let finalize_block = abci::request::FinalizeBlock {
        hash: Hash::try_from([97u8; 32].to_vec()).unwrap(),
        height: 1u32.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: commitments.into_transactions(vec![tx_trigger.to_raw().encode_to_vec().into()]),
        decided_last_commit: CommitInfo {
            votes: vec![],
            round: Round::default(),
        },
        misbehavior: vec![],
    };

    app.finalize_block(finalize_block.clone(), storage.clone())
        .await
        .unwrap();
    assert!(app.recost_mempool, "flag should have been set");
}

#[tokio::test]
async fn do_not_trigger_cleaning() {
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    // create tx which will fail execution and not trigger flag
    // (wrong sudo signer)
    let tx_fail = TransactionBody::builder()
        .actions(vec![
            FeeChange::Transfer(TransferFeeComponents {
                base: 10,
                multiplier: 0,
            })
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&get_alice_signing_key());

    app.mempool
        .insert(
            Arc::new(tx_fail.clone()),
            0,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();

    // trigger with prepare_proposal
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

    assert!(!app.recost_mempool, "flag should start out false");
    app.prepare_proposal(prepare_args, storage.clone())
        .await
        .expect("failing transaction should not cause block to fail");
    assert!(!app.recost_mempool, "flag should not have been set");
}

#[expect(clippy::too_many_lines, reason = "it's a test")]
#[tokio::test]
async fn maintenance_recosting_promotes() {
    // check that transaction promotion from recosting works
    let mut only_alice_funds_genesis_state = proto_genesis_state();
    only_alice_funds_genesis_state.accounts = vec![
        Account {
            address: astria_address_from_hex_string(ALICE_ADDRESS),
            balance: 10u128.pow(19),
        },
        Account {
            address: astria_address_from_hex_string(BOB_ADDRESS),
            balance: 11u128, // transfer fee is 12 at default
        },
    ]
    .into_iter()
    .map(Protobuf::into_raw)
    .collect();

    let (mut app, storage) = initialize_app_with_storage(
        Some(only_alice_funds_genesis_state.try_into().unwrap()),
        vec![],
    )
    .await;
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    // create tx which will not be included in block due to
    // having insufficient funds (transaction will be recosted to enable)
    let tx_fail_recost_funds = TransactionBody::builder()
        .actions(vec![
            Transfer {
                to: astria_address_from_hex_string(CAROL_ADDRESS),
                amount: 1u128,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&get_bob_signing_key());

    let mut bob_funds = HashMap::new();
    bob_funds.insert(nria().into(), 11);
    let mut tx_cost = HashMap::new();
    tx_cost.insert(nria().into(), 13);
    app.mempool
        .insert(
            Arc::new(tx_fail_recost_funds.clone()),
            0,
            bob_funds,
            tx_cost,
        )
        .await
        .unwrap();

    // create tx which will enable recost tx to pass
    let tx_recost = TransactionBody::builder()
        .actions(vec![
            FeeChange::Transfer(TransferFeeComponents {
                base: 10,
                multiplier: 0,
            })
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&get_judy_signing_key());

    let mut judy_funds = HashMap::new();
    judy_funds.insert(nria().into(), 0);
    let mut tx_cost = HashMap::new();
    tx_cost.insert(nria().into(), 0);
    app.mempool
        .insert(Arc::new(tx_recost.clone()), 0, judy_funds, tx_cost)
        .await
        .unwrap();
    assert_eq!(app.mempool.len().await, 2, "two txs in mempool");

    // create block with prepare_proposal
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
    let res = app
        .prepare_proposal(prepare_args, storage.clone())
        .await
        .unwrap();

    assert_eq!(
        res.txs.len(),
        3,
        "only one transaction should've been valid (besides 2 generated txs)"
    );
    assert_eq!(
        app.mempool.len().await,
        2,
        "two txs in mempool; one included in proposal is not yet removed"
    );

    // set dummy hash
    app.executed_proposal_hash = Hash::try_from([97u8; 32].to_vec()).unwrap();

    let process_proposal = abci::request::ProcessProposal {
        hash: app.executed_proposal_hash,
        height: Height::default(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [1u8; 20].to_vec().try_into().unwrap(),
        txs: res.txs.clone(),
        proposed_last_commit: None,
        misbehavior: vec![],
    };
    app.process_proposal(process_proposal, storage.clone())
        .await
        .unwrap();
    assert_eq!(
        app.mempool.len().await,
        2,
        "two txs in mempool; one included in proposal is not
    yet removed"
    );

    // finalize with finalize block
    let finalize_block = abci::request::FinalizeBlock {
        hash: app.executed_proposal_hash,
        height: 1u32.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: res.txs,
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
    assert_eq!(app.mempool.len().await, 1, "recosted tx should remain");

    // mempool re-costing should've occurred to allow other transaction to execute
    let prepare_args = abci::request::PrepareProposal {
        max_tx_bytes: 200_000,
        txs: vec![],
        local_last_commit: None,
        misbehavior: vec![],
        height: 2u8.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: account::Id::new([1u8; 20]),
    };
    let res = app
        .prepare_proposal(prepare_args, storage.clone())
        .await
        .expect("");

    assert_eq!(
        res.txs.len(),
        3,
        "one transaction should've been valid (besides 2 generated txs)"
    );

    // see transfer went through
    assert_eq!(
        app.state_delta
            .get_account_balance(&astria_address_from_hex_string(CAROL_ADDRESS), &nria())
            .await
            .unwrap(),
        1,
        "transfer should've worked"
    );
}

#[expect(clippy::too_many_lines, reason = "it's a test")]
#[tokio::test]
async fn maintenance_funds_added_promotes() {
    // check that transaction promotion from new funds works
    let mut only_alice_funds_genesis_state = proto_genesis_state();
    only_alice_funds_genesis_state.accounts = vec![Account {
        address: astria_address_from_hex_string(ALICE_ADDRESS),
        balance: 10u128.pow(19),
    }]
    .into_iter()
    .map(Protobuf::into_raw)
    .collect();

    let (mut app, storage) = initialize_app_with_storage(
        Some(only_alice_funds_genesis_state.try_into().unwrap()),
        vec![],
    )
    .await;
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    // create tx that will not be included in block due to
    // having no funds (will be sent transfer to then enable)
    let tx_fail_transfer_funds = TransactionBody::builder()
        .actions(vec![
            Transfer {
                to: astria_address_from_hex_string(BOB_ADDRESS),
                amount: 10u128,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&get_carol_signing_key());

    let mut carol_funds = HashMap::new();
    carol_funds.insert(nria().into(), 0);
    let mut tx_cost = HashMap::new();
    tx_cost.insert(nria().into(), 22);
    app.mempool
        .insert(
            Arc::new(tx_fail_transfer_funds.clone()),
            0,
            carol_funds,
            tx_cost,
        )
        .await
        .unwrap();

    // create tx which will enable no funds to pass
    let tx_fund = TransactionBody::builder()
        .actions(vec![
            Transfer {
                to: astria_address_from_hex_string(CAROL_ADDRESS),
                amount: 22u128,
                asset: nria().into(),
                fee_asset: nria().into(),
            }
            .into(),
        ])
        .chain_id("test")
        .try_build()
        .unwrap()
        .sign(&get_alice_signing_key());

    let mut alice_funds = HashMap::new();
    alice_funds.insert(nria().into(), 100);
    let mut tx_cost = HashMap::new();
    tx_cost.insert(nria().into(), 13);
    app.mempool
        .insert(Arc::new(tx_fund.clone()), 0, alice_funds, tx_cost)
        .await
        .unwrap();

    // create block with prepare_proposal
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
    let res = app
        .prepare_proposal(prepare_args, storage.clone())
        .await
        .expect("");

    assert_eq!(
        res.txs.len(),
        3,
        "only one transactions should've been valid (besides 2 generated txs)"
    );

    app.executed_proposal_hash = Hash::try_from([97u8; 32].to_vec()).unwrap();
    let process_proposal = abci::request::ProcessProposal {
        hash: app.executed_proposal_hash,
        height: Height::default(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [1u8; 20].to_vec().try_into().unwrap(),
        txs: res.txs.clone(),
        proposed_last_commit: None,
        misbehavior: vec![],
    };
    app.process_proposal(process_proposal, storage.clone())
        .await
        .unwrap();
    assert_eq!(
        app.mempool.len().await,
        2,
        "two txs in mempool; one included in proposal is not
    yet removed"
    );

    // set dummy hash
    app.executed_proposal_hash = Hash::try_from([97u8; 32].to_vec()).unwrap();

    // finalize with finalize block
    let finalize_block = abci::request::FinalizeBlock {
        hash: app.executed_proposal_hash,
        height: 1u32.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: res.txs,
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

    // transfer should've occurred to allow other transaction to execute
    let prepare_args = abci::request::PrepareProposal {
        max_tx_bytes: 200_000,
        txs: vec![],
        local_last_commit: None,
        misbehavior: vec![],
        height: 2u8.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: account::Id::new([1u8; 20]),
    };
    let res = app
        .prepare_proposal(prepare_args, storage.clone())
        .await
        .expect("");

    assert_eq!(
        res.txs.len(),
        3,
        "only one transactions should've been valid (besides 2 generated txs)"
    );

    // finalize with finalize block
    let finalize_block = abci::request::FinalizeBlock {
        hash: Hash::try_from([97u8; 32].to_vec()).unwrap(),
        height: 1u32.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: res.txs,
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
    // see transfer went through
    assert_eq!(
        app.state_delta
            .get_account_balance(&astria_address_from_hex_string(BOB_ADDRESS), &nria())
            .await
            .unwrap(),
        10,
        "transfer should've worked"
    );
}
