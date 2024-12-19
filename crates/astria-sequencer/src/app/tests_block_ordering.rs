use std::{
    collections::HashMap,
    ops::Deref,
};

use astria_core::{
    protocol::transaction::v1::{
        action::group::Group,
        Transaction,
    },
    Protobuf as _,
};
use bytes::Bytes;
use prost::Message;
use tendermint::{
    abci::request::{
        PrepareProposal,
        ProcessProposal,
    },
    block::Height,
    Hash,
    Time,
};

use super::test_utils::get_alice_signing_key;
use crate::{
    app::{
        benchmark_and_test_utils::{
            initialize_app_with_storage,
            mock_balances,
            mock_tx_cost,
        },
        test_utils::{
            get_bob_signing_key,
            get_judy_signing_key,
            MockTxBuilder,
        },
    },
    proposal::commitment::generate_rollup_datas_commitment,
};

#[tokio::test]
async fn app_process_proposal_ordering_ok() {
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

    // create transactions that should pass with expected ordering
    let txs: Vec<Transaction> = vec![
        MockTxBuilder::new()
            .group(Group::BundleableGeneral)
            .signer(get_alice_signing_key())
            .build()
            .deref()
            .clone(),
        MockTxBuilder::new()
            .group(Group::UnbundleableGeneral)
            .signer(get_bob_signing_key())
            .build()
            .deref()
            .clone(),
        MockTxBuilder::new()
            .group(Group::BundleableSudo)
            .signer(get_judy_signing_key())
            .build()
            .deref()
            .clone(),
        MockTxBuilder::new()
            .group(Group::UnbundleableSudo)
            .nonce(1)
            .signer(get_judy_signing_key())
            .build()
            .deref()
            .clone(),
    ];

    let generated_commitment = generate_rollup_datas_commitment(&txs, HashMap::new());
    let txs = generated_commitment.into_transactions(
        txs.into_iter()
            .map(|tx| tx.to_raw().encode_to_vec().into())
            .collect(),
    );

    let process_proposal = ProcessProposal {
        hash: Hash::Sha256([1; 32]),
        height: 1u32.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs,
        proposed_last_commit: None,
        misbehavior: vec![],
    };

    assert!(
        app.process_proposal(process_proposal.clone(), storage.clone())
            .await
            .is_ok(),
        "process proposal should succeed with expected ordering"
    );
}

#[tokio::test]
async fn app_process_proposal_ordering_fail() {
    // Tests that process proposal will reject blocks that contain transactions that are out of
    // order.
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

    // create transactions that should fail due to incorrect ordering
    let txs: Vec<Transaction> = vec![
        MockTxBuilder::new()
            .group(Group::UnbundleableGeneral)
            .signer(get_bob_signing_key())
            .build()
            .deref()
            .clone(),
        MockTxBuilder::new()
            .group(Group::BundleableGeneral)
            .signer(get_alice_signing_key())
            .build()
            .deref()
            .clone(),
    ];

    let generated_commitment = generate_rollup_datas_commitment(&txs, HashMap::new());
    let txs = generated_commitment.into_transactions(
        txs.into_iter()
            .map(|tx| tx.to_raw().encode_to_vec().into())
            .collect(),
    );

    let process_proposal = ProcessProposal {
        hash: Hash::default(),
        height: 1u32.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs,
        proposed_last_commit: None,
        misbehavior: vec![],
    };

    let result = app
        .process_proposal(process_proposal.clone(), storage.clone())
        .await
        .expect_err("expected ordering error");
    assert!(
        format!("{result:?}").contains("transactions have incorrect transaction group ordering"),
        "process proposal should fail due to incorrect ordering"
    );
}

#[tokio::test]
async fn app_prepare_proposal_account_block_misordering_ok() {
    // This test ensures that if an account has transactions that are valid eventually but are
    // invalid in the same block that they aren't rejected but instead are included in multiple
    // blocks.
    //
    // For example, if an account sends transactions:
    // tx_1: {nonce:0, action_group_type:UnbundleableGeneral}
    // tx_2: {nonce:1, action_group_type:BundleableGeneral}
    // If these were included in the same block tx_2 would be placed before tx_1 because its group
    // has a higher priority even though it will fail execution due to having the wrong nonce.
    //
    // The block building process should handle this in a way that allows the transactions to
    // both eventually be included.
    let (mut app, storage) = initialize_app_with_storage(None, vec![]).await;

    // create transactions that should fail due to incorrect ordering if both are included in the
    // same block
    let tx_0 = MockTxBuilder::new()
        .group(Group::UnbundleableGeneral)
        .signer(get_alice_signing_key())
        .build();
    let tx_1 = MockTxBuilder::new()
        .group(Group::BundleableGeneral)
        .nonce(1)
        .signer(get_alice_signing_key())
        .build();

    app.mempool
        .insert(tx_0.clone(), 0, mock_balances(0, 0), mock_tx_cost(0, 0, 0))
        .await
        .unwrap();

    app.mempool
        .insert(tx_1.clone(), 0, mock_balances(0, 0), mock_tx_cost(0, 0, 0))
        .await
        .unwrap();

    let prepare_args = PrepareProposal {
        max_tx_bytes: 600_000,
        txs: vec![],
        local_last_commit: None,
        misbehavior: vec![],
        height: Height::default(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [88u8; 20].to_vec().try_into().unwrap(),
    };

    let prepare_proposal_result = app
        .prepare_proposal(prepare_args, storage.clone())
        .await
        .expect("incorrect account ordering shouldn't cause blocks to fail");

    assert_eq!(
        prepare_proposal_result.txs[2],
        Into::<Bytes>::into(tx_0.to_raw().encode_to_vec()),
        "expected to contain first transaction"
    );

    app.mempool.run_maintenance(&app.state_delta, false).await;
    assert_eq!(
        app.mempool.len().await,
        1,
        "mempool should contain 2nd transaction still"
    );

    // commit state for next prepare proposal
    app.prepare_commit(storage.clone()).await.unwrap();
    app.commit(storage.clone()).await;

    let prepare_args = PrepareProposal {
        max_tx_bytes: 600_000,
        txs: vec![],
        local_last_commit: None,
        misbehavior: vec![],
        height: 1u32.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [88u8; 20].to_vec().try_into().unwrap(),
    };
    let prepare_proposal_result = app
        .prepare_proposal(prepare_args, storage.clone())
        .await
        .expect("incorrect account ordering shouldn't cause blocks to fail");

    assert_eq!(
        prepare_proposal_result.txs[2],
        Into::<Bytes>::into(tx_1.to_raw().encode_to_vec()),
        "expected to contain second transaction"
    );

    app.mempool.run_maintenance(&app.state_delta, false).await;
    assert_eq!(app.mempool.len().await, 0, "mempool should be empty");
}
