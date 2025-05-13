use std::{
    collections::HashMap,
    sync::Arc,
};

use astria_core::{
    crypto::SigningKey,
    primitive::v1::RollupId,
    protocol::transaction::v1::action::{
        group::Group,
        FeeAssetChange,
        InitBridgeAccount,
        SudoAddressChange,
    },
};
use bytes::Bytes;
use tendermint::{
    abci::{
        request::{
            PrepareProposal,
            ProcessProposal,
        },
        types::{
            CommitInfo,
            ExtendedCommitInfo,
        },
    },
    Hash,
    Time,
};

use crate::{
    checked_transaction::CheckedTransaction,
    test_utils::{
        dummy_balances,
        dummy_tx_costs,
        nria,
        transactions_with_extended_commit_info_and_commitments,
        Fixture,
        ALICE,
        BOB,
        CAROL_ADDRESS,
        SUDO,
    },
};

async fn new_bundleable_general_tx(
    fixture: &Fixture,
    signer: SigningKey,
    nonce: u32,
) -> Arc<CheckedTransaction> {
    let tx = fixture
        .checked_tx_builder()
        .with_rollup_data_submission(vec![1, 2, 3])
        .with_nonce(nonce)
        .with_signer(signer)
        .build()
        .await;
    assert_eq!(tx.group(), Group::BundleableGeneral);
    tx
}

async fn new_unbundleable_general_tx(
    fixture: &Fixture,
    signer: SigningKey,
    nonce: u32,
) -> Arc<CheckedTransaction> {
    let tx = fixture
        .checked_tx_builder()
        .with_action(InitBridgeAccount {
            rollup_id: RollupId::from_unhashed_bytes("rollup-id"),
            asset: nria().into(),
            fee_asset: nria().into(),
            sudo_address: None,
            withdrawer_address: None,
        })
        .with_nonce(nonce)
        .with_signer(signer)
        .build()
        .await;
    assert_eq!(tx.group(), Group::UnbundleableGeneral);
    tx
}

async fn new_bundleable_sudo_tx(fixture: &Fixture, nonce: u32) -> Arc<CheckedTransaction> {
    let tx = fixture
        .checked_tx_builder()
        .with_action(FeeAssetChange::Addition("other_asset".parse().unwrap()))
        .with_nonce(nonce)
        .with_signer(SUDO.clone())
        .build()
        .await;
    assert_eq!(tx.group(), Group::BundleableSudo);
    tx
}

async fn new_unbundleable_sudo_tx(fixture: &Fixture, nonce: u32) -> Arc<CheckedTransaction> {
    let tx = fixture
        .checked_tx_builder()
        .with_action(SudoAddressChange {
            new_address: *CAROL_ADDRESS,
        })
        .with_nonce(nonce)
        .with_signer(SUDO.clone())
        .build()
        .await;
    assert_eq!(tx.group(), Group::UnbundleableSudo);
    tx
}

#[tokio::test]
async fn app_process_proposal_ordering_ok() {
    let fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    // create transactions that should pass with expected ordering
    let txs = vec![
        new_bundleable_general_tx(&fixture, ALICE.clone(), 0).await,
        new_unbundleable_general_tx(&fixture, BOB.clone(), 0).await,
        new_bundleable_sudo_tx(&fixture, 0).await,
        new_unbundleable_sudo_tx(&fixture, 1).await,
    ];

    let process_proposal = ProcessProposal {
        hash: Hash::Sha256([1; 32]),
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

    let (mut app, storage) = fixture.destructure();
    assert!(
        app.process_proposal(process_proposal, storage)
            .await
            .is_ok(),
        "process proposal should succeed with expected ordering"
    );
}

#[tokio::test]
async fn app_process_proposal_ordering_fail() {
    // Tests that process proposal will reject blocks that contain transactions that are out of
    // order.
    let fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    // create transactions that should fail due to incorrect ordering
    let txs = vec![
        new_unbundleable_general_tx(&fixture, BOB.clone(), 0).await,
        new_bundleable_general_tx(&fixture, ALICE.clone(), 0).await,
    ];

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

    let (mut app, storage) = fixture.destructure();
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
    let fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    // create transactions that should fail due to incorrect ordering if both are included in the
    // same block
    let tx_0 = new_unbundleable_general_tx(&fixture, ALICE.clone(), 0).await;
    let tx_1 = new_bundleable_general_tx(&fixture, ALICE.clone(), 1).await;

    let (mut app, storage) = fixture.destructure();
    app.mempool
        .insert(
            tx_0.clone(),
            0,
            &dummy_balances(0, 0),
            dummy_tx_costs(0, 0, 0),
        )
        .await
        .unwrap();

    app.mempool
        .insert(
            tx_1.clone(),
            0,
            &dummy_balances(0, 0),
            dummy_tx_costs(0, 0, 0),
        )
        .await
        .unwrap();

    let prepare_args = PrepareProposal {
        max_tx_bytes: 600_000,
        txs: vec![],
        local_last_commit: Some(ExtendedCommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [88u8; 20].to_vec().try_into().unwrap(),
    };

    let prepare_proposal_result = app
        .prepare_proposal(prepare_args, storage.clone())
        .await
        .expect("incorrect account ordering shouldn't cause blocks to fail");

    assert_eq!(
        Bytes::from(prepare_proposal_result.txs.last().unwrap().to_vec()),
        tx_0.encoded_bytes(),
        "expected to contain first transaction"
    );

    app.mempool
        .run_maintenance(&app.state, false, HashMap::new(), 0)
        .await;
    assert_eq!(
        app.mempool.len().await,
        1,
        "mempool should contain 2nd transaction still"
    );

    // commit state for next prepare proposal
    app.prepare_commit(storage.clone(), Vec::new())
        .await
        .unwrap();
    app.commit(storage.clone()).await.unwrap();

    let prepare_args = PrepareProposal {
        max_tx_bytes: 600_000,
        txs: vec![],
        local_last_commit: Some(ExtendedCommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
        height: height.increment(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [88u8; 20].to_vec().try_into().unwrap(),
    };
    let prepare_proposal_result = app
        .prepare_proposal(prepare_args, storage.clone())
        .await
        .expect("incorrect account ordering shouldn't cause blocks to fail");

    assert_eq!(
        Bytes::from(prepare_proposal_result.txs.last().unwrap().to_vec()),
        tx_1.encoded_bytes(),
        "expected to contain second transaction"
    );

    app.mempool
        .run_maintenance(&app.state, false, HashMap::new(), 0)
        .await;
    assert_eq!(app.mempool.len().await, 0, "mempool should be empty");
}
