use std::collections::HashMap;

use astria_core::protocol::{
    fees::v1::FeeComponents,
    transaction::v1::action::{
        FeeChange,
        Transfer,
    },
};
use tendermint::{
    abci::{
        request::FinalizeBlock,
        types::CommitInfo,
    },
    account,
    block::Round,
    Hash,
    Time,
};

use super::*;
use crate::{
    accounts::StateReadExt as _,
    test_utils::{
        dummy_balances,
        dummy_tx_costs,
        nria,
        Fixture,
        ALICE,
        ALICE_ADDRESS,
        BOB,
        BOB_ADDRESS,
        CAROL,
        CAROL_ADDRESS,
        TEN_QUINTILLION,
    },
};

#[tokio::test]
async fn trigger_cleaning() {
    // check that cleaning is triggered by the prepare, process, and finalize block flows
    let mut fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    // create tx which will cause mempool cleaning flag to be set
    let tx_trigger = fixture
        .checked_tx_builder()
        .with_action(FeeChange::Transfer(FeeComponents::new(10, 0)))
        .build()
        .await;

    fixture
        .mempool()
        .insert(
            tx_trigger.clone(),
            0,
            &dummy_balances(0, 0),
            dummy_tx_costs(0, 0, 0),
        )
        .await
        .unwrap();

    assert!(!fixture.app.recost_mempool, "flag should start out false");

    // trigger with prepare_proposal
    let prepare_args = PrepareProposal {
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

    fixture
        .app
        .prepare_proposal(prepare_args, fixture.storage())
        .await
        .expect("fee change with correct signer should pass prepare proposal");
    assert!(fixture.app.recost_mempool, "flag should have been set");

    // manually reset to trigger again
    fixture.app.recost_mempool = false;

    // trigger with process_proposal
    let txs = transactions_with_extended_commit_info_and_commitments(height, &[tx_trigger], None);
    let process_proposal = ProcessProposal {
        hash: Hash::try_from([99u8; 32].to_vec()).unwrap(),
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: txs.clone(),
        proposed_last_commit: Some(CommitInfo {
            votes: vec![],
            round: Round::default(),
        }),
        misbehavior: vec![],
    };

    fixture
        .app
        .process_proposal(process_proposal, fixture.storage())
        .await
        .unwrap();
    assert!(fixture.app.recost_mempool, "flag should have been set");

    // trigger with finalize block
    fixture.app.recost_mempool = false;

    let finalize_block = FinalizeBlock {
        hash: Hash::try_from([97u8; 32].to_vec()).unwrap(),
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs,
        decided_last_commit: CommitInfo {
            votes: vec![],
            round: Round::default(),
        },
        misbehavior: vec![],
    };

    fixture
        .app
        .finalize_block(finalize_block, fixture.storage())
        .await
        .unwrap();
    assert!(fixture.app.recost_mempool, "flag should have been set");
}

#[tokio::test]
async fn do_not_trigger_cleaning() {
    let mut fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    // create tx which will fail execution and not trigger flag
    // (change sudo to Alice for checked tx construction, but don't commit the change to sudo
    // address, so `prepare_proposal` call uses `storage` with sudo address as `SUDO`)
    fixture
        .state_mut()
        .put_sudo_address(*ALICE_ADDRESS)
        .unwrap();
    let tx_fail = fixture
        .checked_tx_builder()
        .with_action(FeeChange::Transfer(FeeComponents::new(10, 0)))
        .with_signer(ALICE.clone())
        .build()
        .await;

    fixture
        .mempool()
        .insert(tx_fail, 0, &dummy_balances(0, 0), dummy_tx_costs(0, 0, 0))
        .await
        .unwrap();

    // trigger with prepare_proposal
    let prepare_args = PrepareProposal {
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

    assert!(!fixture.app.recost_mempool, "flag should start out false");
    fixture
        .app
        .prepare_proposal(prepare_args, fixture.storage())
        .await
        .expect("failing transaction should not cause block to fail");
    assert!(!fixture.app.recost_mempool, "flag should not have been set");
}

#[expect(clippy::too_many_lines, reason = "it's a test")]
#[tokio::test]
async fn maintenance_recosting_promotes() {
    // check that transaction promotion from recosting works
    let mut fixture = Fixture::uninitialized(None).await;
    // Provide Alice with normal balance
    // Provide Bob with just enough to cover the costs of a Transfer of 1 nria after the Transfer
    // fee is reduced to 1 nria.
    fixture
        .chain_initializer()
        .with_genesis_accounts(vec![(*ALICE_ADDRESS, TEN_QUINTILLION), (*BOB_ADDRESS, 2)])
        .init()
        .await;

    let height = fixture.run_until_aspen_applied().await;

    // create tx which will not be included in block due to
    // having insufficient funds (transaction will be recosted to enable)
    let tx_fail_recost_funds = fixture
        .checked_tx_builder()
        .with_action(Transfer {
            to: *CAROL_ADDRESS,
            amount: 1,
            asset: nria().into(),
            fee_asset: nria().into(),
        })
        .with_signer(BOB.clone())
        .build()
        .await;

    let mut bob_funds = HashMap::new();
    bob_funds.insert(nria().into(), 2);
    let mut tx_cost = HashMap::new();
    tx_cost.insert(nria().into(), 3);
    let mempool = fixture.mempool();
    mempool
        .insert(tx_fail_recost_funds, 0, &bob_funds, tx_cost)
        .await
        .unwrap();

    // create tx which will enable recost tx to pass
    let tx_recost = fixture
        .checked_tx_builder()
        .with_action(FeeChange::Transfer(FeeComponents::<Transfer>::new(1, 0)))
        .build()
        .await;

    let mut sudo_funds = HashMap::new();
    sudo_funds.insert(nria().into(), 0);
    let mut tx_cost = HashMap::new();
    tx_cost.insert(nria().into(), 0);
    mempool
        .insert(tx_recost, 0, &sudo_funds, tx_cost)
        .await
        .unwrap();
    assert_eq!(mempool.len().await, 2, "two txs in mempool");

    // create block with prepare_proposal
    let prepare_args = PrepareProposal {
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
    let res = fixture
        .app
        .prepare_proposal(prepare_args, fixture.storage())
        .await
        .unwrap();

    assert_eq!(
        res.txs.len(),
        4,
        "only one transaction should've been valid (besides 3 generated txs)"
    );
    assert_eq!(
        mempool.len().await,
        2,
        "two txs in mempool; one included in proposal is not yet removed"
    );

    let hash = Hash::Sha256([97u8; 32]);
    let process_proposal = ProcessProposal {
        hash,
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [1u8; 20].to_vec().try_into().unwrap(),
        txs: res.txs.clone(),
        proposed_last_commit: Some(CommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
    };
    fixture
        .app
        .process_proposal(process_proposal, fixture.storage())
        .await
        .unwrap();
    assert_eq!(
        mempool.len().await,
        2,
        "two txs in mempool; one included in proposal is not yet removed"
    );

    // finalize with finalize block
    let finalize_block = FinalizeBlock {
        hash,
        height,
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

    fixture
        .app
        .finalize_block(finalize_block.clone(), fixture.storage())
        .await
        .unwrap();
    fixture.app.commit(fixture.storage()).await.unwrap();
    assert_eq!(mempool.len().await, 1, "recosted tx should remain");

    // mempool re-costing should've occurred to allow other transaction to execute
    let next_height = height.increment();
    let prepare_args = PrepareProposal {
        max_tx_bytes: 200_000,
        txs: vec![],
        local_last_commit: Some(ExtendedCommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
        height: next_height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: account::Id::new([1u8; 20]),
    };
    let res = fixture
        .app
        .prepare_proposal(prepare_args, fixture.storage())
        .await
        .unwrap();

    assert_eq!(
        res.txs.len(),
        4,
        "only one transaction should've been valid (besides 3 generated txs)"
    );

    // see transfer went through
    assert_eq!(
        fixture
            .state()
            .get_account_balance(&*CAROL_ADDRESS, &nria())
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
    let mut fixture = Fixture::uninitialized(None).await;
    // Alice is the only funded account at genesis.
    fixture
        .chain_initializer()
        .with_genesis_accounts(vec![(*ALICE_ADDRESS, TEN_QUINTILLION)])
        .init()
        .await;

    let height = fixture.run_until_aspen_applied().await;

    // create tx that will not be included in block due to
    // having no funds (will be sent transfer to then enable)
    let tx_fail_transfer_funds = fixture
        .checked_tx_builder()
        .with_action(Transfer {
            to: *BOB_ADDRESS,
            amount: 10,
            asset: nria().into(),
            fee_asset: nria().into(),
        })
        .with_signer(CAROL.clone())
        .build()
        .await;

    let mut carol_funds = HashMap::new();
    carol_funds.insert(nria().into(), 0);
    let mut tx_cost = HashMap::new();
    tx_cost.insert(nria().into(), 22);
    let mempool = fixture.mempool();
    mempool
        .insert(tx_fail_transfer_funds, 0, &carol_funds, tx_cost)
        .await
        .unwrap();

    // create tx which will enable no funds to pass
    let tx_fund = fixture
        .checked_tx_builder()
        .with_action(Transfer {
            to: *CAROL_ADDRESS,
            amount: 22,
            asset: nria().into(),
            fee_asset: nria().into(),
        })
        .with_signer(ALICE.clone())
        .build()
        .await;

    let mut alice_funds = HashMap::new();
    alice_funds.insert(nria().into(), 100);
    let mut tx_cost = HashMap::new();
    tx_cost.insert(nria().into(), 13);
    mempool
        .insert(tx_fund, 0, &alice_funds, tx_cost)
        .await
        .unwrap();

    // create block with prepare_proposal
    let prepare_args = PrepareProposal {
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
    let res = fixture
        .app
        .prepare_proposal(prepare_args, fixture.storage())
        .await
        .unwrap();

    assert_eq!(
        res.txs.len(),
        4,
        "only one transactions should've been valid (besides 3 generated txs)"
    );

    let hash = Hash::Sha256([97u8; 32]);
    let process_proposal = ProcessProposal {
        hash,
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [1u8; 20].to_vec().try_into().unwrap(),
        txs: res.txs.clone(),
        proposed_last_commit: Some(CommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
    };
    fixture
        .app
        .process_proposal(process_proposal, fixture.storage())
        .await
        .unwrap();
    assert_eq!(
        mempool.len().await,
        2,
        "two txs in mempool; one included in proposal is not yet removed"
    );

    // finalize with finalize block
    let finalize_block = FinalizeBlock {
        hash,
        height,
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
    fixture
        .app
        .finalize_block(finalize_block.clone(), fixture.storage())
        .await
        .unwrap();
    fixture.app.commit(fixture.storage()).await.unwrap();

    // transfer should've occurred to allow other transaction to execute
    let next_height = height.increment();
    let prepare_args = PrepareProposal {
        max_tx_bytes: 200_000,
        txs: vec![],
        local_last_commit: Some(ExtendedCommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
        height: next_height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: account::Id::new([1u8; 20]),
    };
    let res = fixture
        .app
        .prepare_proposal(prepare_args, fixture.storage())
        .await
        .unwrap();

    assert_eq!(
        res.txs.len(),
        4,
        "only one transactions should've been valid (besides 3 generated txs)"
    );

    // finalize with finalize block
    let finalize_block = FinalizeBlock {
        hash: Hash::try_from([97u8; 32].to_vec()).unwrap(),
        height: next_height,
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
    fixture
        .app
        .finalize_block(finalize_block.clone(), fixture.storage())
        .await
        .unwrap();
    fixture.app.commit(fixture.storage()).await.unwrap();
    // see transfer went through
    assert_eq!(
        fixture
            .state()
            .get_account_balance(&*BOB_ADDRESS, &nria())
            .await
            .unwrap(),
        10,
        "transfer should've worked"
    );
}
