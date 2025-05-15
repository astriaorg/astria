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
        transaction::v1::action::{
            BridgeLock,
            RollupDataSubmission,
            SudoAddressChange,
            Transfer,
        },
    },
    sequencerblock::v1::block::{
        Deposit,
        RollupData,
    },
    upgrades::test_utils::UpgradesBuilder,
};
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
    assets::StateReadExt as _,
    authority::{
        StateReadExt as _,
        StateWriteExt as _,
        ValidatorSet,
    },
    bridge::StateWriteExt as _,
    fees::StateReadExt as _,
    grpc::StateReadExt as _,
    oracles::price_feed::oracle::state_ext::StateWriteExt as _,
    proposal::commitment::generate_rollup_datas_commitment,
    test_utils::{
        assert_error_contains,
        astria_address,
        dummy_balances,
        dummy_tx_costs,
        nria,
        transactions_with_extended_commit_info_and_commitments,
        Fixture,
        ALICE,
        ALICE_ADDRESS_BYTES,
        BOB,
        BOB_ADDRESS,
        BOB_ADDRESS_BYTES,
        CAROL,
        CAROL_ADDRESS_BYTES,
        SUDO,
        SUDO_ADDRESS_BYTES,
    },
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
    let mut fixture = Fixture::uninitialized(None).await;
    fixture.chain_initializer().init().await;
    assert_eq!(fixture.state().get_block_height().await.unwrap(), 0);

    for Account {
        address,
        balance,
    } in fixture.genesis_app_state().accounts()
    {
        assert_eq!(*balance, fixture.get_nria_balance(address).await);
    }

    assert_eq!(
        fixture.state().get_native_asset().await.unwrap(),
        Some("nria".parse::<TracePrefixed>().unwrap()),
    );
}

#[tokio::test]
async fn app_pre_execute_transactions() {
    let mut fixture = Fixture::uninitialized(None).await;
    fixture.chain_initializer().init().await;

    let block_data = BlockData {
        misbehavior: vec![],
        height: 1u8.into(),
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: account::Id::try_from([0u8; 20].to_vec()).unwrap(),
    };

    fixture
        .app
        .pre_execute_transactions(block_data.clone())
        .await
        .unwrap();
    assert_eq!(fixture.state().get_block_height().await.unwrap(), 1);
    assert_eq!(
        fixture.state().get_block_timestamp().await.unwrap(),
        block_data.time
    );
}

#[tokio::test]
async fn app_begin_block_remove_byzantine_validators() {
    use tendermint::abci::types;

    let upgrades = Some(UpgradesBuilder::new().set_aspen(Some(100)).build());
    let mut fixture = Fixture::uninitialized(upgrades).await;
    fixture.chain_initializer().init().await;

    let validator_set = fixture
        .app
        .state
        .pre_aspen_get_validator_set()
        .await
        .unwrap();
    assert_eq!(validator_set.len(), 3);
    let mut total_voting_power = validator_set.get(&*ALICE_ADDRESS_BYTES).unwrap().power;
    total_voting_power = total_voting_power
        .checked_add(validator_set.get(&*BOB_ADDRESS_BYTES).unwrap().power)
        .unwrap();
    total_voting_power = total_voting_power
        .checked_add(validator_set.get(&*CAROL_ADDRESS_BYTES).unwrap().power)
        .unwrap();

    let misbehavior = types::Misbehavior {
        kind: types::MisbehaviorKind::Unknown,
        validator: Validator {
            address: *CAROL_ADDRESS_BYTES,
            power: 0_u32.into(),
        },
        height: Height::default(),
        time: Time::now(),
        total_voting_power: total_voting_power.into(),
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

    fixture.app.begin_block(&begin_block).await.unwrap();

    // assert that Carol is removed
    let validator_set = fixture
        .app
        .state
        .pre_aspen_get_validator_set()
        .await
        .unwrap();
    assert_eq!(validator_set.len(), 2);
    assert!(validator_set.get(&*CAROL_ADDRESS_BYTES).is_none());
}

#[tokio::test]
async fn app_commit() {
    let mut fixture = Fixture::uninitialized(None).await;
    fixture.chain_initializer().init().await;

    assert_eq!(
        fixture
            .storage()
            .latest_snapshot()
            .get_block_height()
            .await
            .unwrap(),
        0
    );

    // Write block height 1 to the App's state delta.
    fixture.app.state_mut().put_block_height(1).unwrap();
    assert_eq!(fixture.app.state().get_block_height().await.unwrap(), 1);

    // The latest snapshot should still have block height 0.
    assert_eq!(
        fixture
            .storage()
            .latest_snapshot()
            .get_block_height()
            .await
            .unwrap(),
        0
    );

    // Commit should write the changes to the underlying storage.
    fixture
        .app
        .prepare_commit(fixture.storage(), HashSet::new())
        .await
        .unwrap();
    fixture.app.commit(fixture.storage()).await.unwrap();
    assert_eq!(
        fixture
            .storage()
            .latest_snapshot()
            .get_block_height()
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn app_transfer_block_fees_to_sudo() {
    let mut fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    // transfer funds from Alice to Bob; use native token for fee payment
    let amount = 333_333;
    let tx = fixture
        .checked_tx_builder()
        .with_action(Transfer {
            to: *BOB_ADDRESS,
            amount,
            asset: nria().into(),
            fee_asset: nria().into(),
        })
        .with_signer(ALICE.clone())
        .build()
        .await;

    let proposer_address: account::Id = [99u8; 20].to_vec().try_into().unwrap();
    let finalize_block = abci::request::FinalizeBlock {
        hash: Hash::try_from([0u8; 32].to_vec()).unwrap(),
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address,
        txs: transactions_with_extended_commit_info_and_commitments(height, &[tx], None),
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
    fixture.app.commit(fixture.storage()).await.unwrap();

    // assert that transaction fees were transferred to the block proposer
    let transfer_base_fee = fixture
        .state()
        .get_fees::<Transfer>()
        .await
        .expect("should not error fetching transfer fees")
        .expect("transfer fees should be stored")
        .base();
    assert_eq!(
        fixture
            .state()
            .get_account_balance(&*SUDO_ADDRESS_BYTES, &nria())
            .await
            .unwrap(),
        transfer_base_fee,
    );
    assert_eq!(fixture.state().get_block_fees().len(), 0);
}

#[expect(clippy::too_many_lines, reason = "it's a test")]
#[tokio::test]
async fn app_create_sequencer_block_with_sequenced_data_and_deposits() {
    let mut fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    let bridge_address = astria_address(&[99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let starting_index_of_action = 0;

    fixture
        .state_mut()
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    fixture
        .state_mut()
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
    fixture
        .state_mut()
        .put_deposits(
            &[32u8; 32],
            HashMap::from_iter([(rollup_id, vec![old_deposit])]),
        )
        .unwrap();
    fixture
        .app
        .prepare_commit(fixture.storage(), HashSet::new())
        .await
        .unwrap();
    fixture.app.commit(fixture.storage()).await.unwrap();

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

    let tx = fixture
        .checked_tx_builder()
        .with_action(lock_action)
        .with_action(rollup_data_submission)
        .with_signer(ALICE.clone())
        .build()
        .await;

    let expected_deposit = Deposit {
        bridge_address,
        rollup_id,
        amount,
        asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
        source_transaction_id: *tx.id(),
        source_action_index: starting_index_of_action,
    };
    let deposits = HashMap::from_iter(vec![(rollup_id, vec![expected_deposit.clone()])]);

    let finalize_block = abci::request::FinalizeBlock {
        hash: Hash::try_from([0u8; 32].to_vec()).unwrap(),
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: transactions_with_extended_commit_info_and_commitments(height, &[tx], Some(deposits)),
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
    fixture.app.commit(fixture.storage()).await.unwrap();

    let block = fixture
        .state()
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
#[expect(clippy::too_many_lines, reason = "it's a test")]
async fn app_execution_results_match_proposal_vs_after_proposal() {
    let mut fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    let bridge_address = astria_address(&[99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let asset = nria().clone();
    let starting_index_of_action = 0;

    fixture
        .state_mut()
        .put_bridge_account_rollup_id(&bridge_address, rollup_id)
        .unwrap();
    fixture
        .state_mut()
        .put_bridge_account_ibc_asset(&bridge_address, &asset)
        .unwrap();
    fixture
        .app
        .prepare_commit(fixture.storage(), HashSet::new())
        .await
        .unwrap();
    fixture.app.commit(fixture.storage()).await.unwrap();

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

    let tx = fixture
        .checked_tx_builder()
        .with_action(lock_action)
        .with_action(rollup_data_submission)
        .with_signer(ALICE.clone())
        .build()
        .await;

    let expected_deposit = Deposit {
        bridge_address,
        rollup_id,
        amount,
        asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
        source_transaction_id: *tx.id(),
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
            &[tx.clone()],
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
    let finalize_block_result = fixture
        .app
        .finalize_block(finalize_block.clone(), fixture.storage())
        .await
        .unwrap();

    // don't commit the result, now call prepare_proposal with the same data.
    // this will reset the app state.
    // this simulates executing the same block as a validator (specifically the proposer).
    let mempool = fixture.mempool();
    mempool
        .insert(tx, 0, &dummy_balances(0, 0), dummy_tx_costs(0, 0, 0))
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

    let prepare_proposal_result = fixture
        .app
        .prepare_proposal(prepare_proposal, fixture.storage())
        .await
        .unwrap();
    assert_eq!(prepare_proposal_result.txs, finalize_block.txs);

    mempool
        .run_maintenance(fixture.state(), false, &HashSet::new(), 0)
        .await;

    assert_eq!(mempool.len().await, 0);

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

    fixture
        .app
        .process_proposal(process_proposal.clone(), fixture.storage())
        .await
        .unwrap();

    let finalize_block_after_prepare_proposal_result = fixture
        .app
        .finalize_block(finalize_block.clone(), fixture.storage())
        .await
        .unwrap();
    assert_eq!(
        finalize_block_after_prepare_proposal_result.app_hash,
        finalize_block_result.app_hash
    );

    // reset the app state and call process_proposal - should execute the block.
    // this simulates executing the block as a non-proposer validator.
    fixture.app.update_state_for_new_round(&fixture.storage());
    fixture
        .app
        .process_proposal(process_proposal, fixture.storage())
        .await
        .unwrap();
    let finalize_block_after_process_proposal_result = fixture
        .app
        .finalize_block(finalize_block, fixture.storage())
        .await
        .unwrap();

    assert_eq!(
        finalize_block_after_process_proposal_result.app_hash,
        finalize_block_result.app_hash
    );
}

#[tokio::test]
async fn app_prepare_proposal_cometbft_max_bytes_overflow_ok() {
    let mut fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    // create txs which will cause cometBFT overflow
    let tx_pass = fixture
        .checked_tx_builder()
        .with_action(RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 100_000]),
            fee_asset: nria().into(),
        })
        .with_signer(ALICE.clone())
        .build()
        .await;

    let tx_overflow = fixture
        .checked_tx_builder()
        .with_action(RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 100_000]),
            fee_asset: nria().into(),
        })
        .with_nonce(1)
        .with_signer(ALICE.clone())
        .build()
        .await;

    let mempool = fixture.mempool();
    mempool
        .insert(tx_pass, 0, &dummy_balances(0, 0), dummy_tx_costs(0, 0, 0))
        .await
        .unwrap();
    mempool
        .insert(
            tx_overflow,
            0,
            &dummy_balances(0, 0),
            dummy_tx_costs(0, 0, 0),
        )
        .await
        .unwrap();

    // send to prepare_proposal
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

    let result = fixture
        .app
        .prepare_proposal(prepare_args, fixture.storage())
        .await
        .expect("too large transactions should not cause prepare proposal to fail");

    // run maintenance to clear out transactions
    mempool
        .run_maintenance(fixture.state(), false, &HashSet::new(), 0)
        .await;

    // see only first tx made it in
    assert_eq!(
        result.txs.len(),
        4,
        "total transaction length should be four, including the extended commit info, two \
         commitments and the one tx that fit"
    );
    assert_eq!(
        mempool.len().await,
        1,
        "mempool should have re-added the tx that was too large"
    );
}

#[tokio::test]
async fn app_prepare_proposal_sequencer_max_bytes_overflow_ok() {
    let mut fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    // create txs which will cause sequencer overflow (max is currently 256_000 bytes)
    let tx_pass = fixture
        .checked_tx_builder()
        .with_action(RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 200_000]),
            fee_asset: nria().into(),
        })
        .with_signer(ALICE.clone())
        .build()
        .await;
    let tx_overflow = fixture
        .checked_tx_builder()
        .with_action(RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 100_000]),
            fee_asset: nria().into(),
        })
        .with_nonce(1)
        .with_signer(ALICE.clone())
        .build()
        .await;

    let mempool = fixture.mempool();
    mempool
        .insert(tx_pass, 0, &dummy_balances(0, 0), dummy_tx_costs(0, 0, 0))
        .await
        .unwrap();
    mempool
        .insert(
            tx_overflow,
            0,
            &dummy_balances(0, 0),
            dummy_tx_costs(0, 0, 0),
        )
        .await
        .unwrap();

    // send to prepare_proposal
    let prepare_args = PrepareProposal {
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

    let result = fixture
        .app
        .prepare_proposal(prepare_args, fixture.storage())
        .await
        .expect("too large transactions should not cause prepare proposal to fail");

    // run maintenance to clear out transactions
    mempool
        .run_maintenance(fixture.state(), false, &HashSet::new(), 0)
        .await;

    // see only first tx made it in
    assert_eq!(
        result.txs.len(),
        4,
        "total transaction length should be four, including the extended commit info, two \
         commitments and the one tx that fit"
    );
    assert_eq!(
        mempool.len().await,
        1,
        "mempool should have re-added the tx that was too large"
    );
}

#[tokio::test]
async fn app_process_proposal_sequencer_max_bytes_overflow_fail() {
    let mut fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    // create txs which will cause sequencer overflow (max is currently 256_000 bytes)
    let tx_pass = fixture
        .checked_tx_builder()
        .with_action(RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 200_000]),
            fee_asset: nria().into(),
        })
        .with_signer(ALICE.clone())
        .build()
        .await;
    let tx_overflow = fixture
        .checked_tx_builder()
        .with_action(RollupDataSubmission {
            rollup_id: RollupId::from([1u8; 32]),
            data: Bytes::copy_from_slice(&[1u8; 100_000]),
            fee_asset: nria().into(),
        })
        .with_nonce(1)
        .with_signer(ALICE.clone())
        .build()
        .await;

    let txs = vec![tx_pass, tx_overflow];
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

    let result = fixture
        .app
        .process_proposal(process_proposal.clone(), fixture.storage())
        .await
        .expect_err("expected max sequenced data limit error");

    assert!(
        format!("{result:?}").contains("max block sequenced data limit passed"),
        "process proposal should fail due to max sequenced data limit"
    );
}

#[tokio::test]
async fn app_process_proposal_transaction_fails_to_execute_fails() {
    let mut fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();

    // Create txs which will cause transaction execution failure.
    // Temporarily make Alice the sudo address to construct the checked tx.
    fixture
        .state_mut()
        .put_sudo_address(*ALICE_ADDRESS_BYTES)
        .unwrap();
    let tx_fail = fixture
        .checked_tx_builder()
        .with_action(SudoAddressChange {
            new_address: *BOB_ADDRESS,
        })
        .with_signer(ALICE.clone())
        .build()
        .await;
    fixture
        .state_mut()
        .put_sudo_address(*SUDO_ADDRESS_BYTES)
        .unwrap();

    let process_proposal = ProcessProposal {
        hash: Hash::default(),
        height,
        time: Time::now(),
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: transactions_with_extended_commit_info_and_commitments(height, &[tx_fail], None),
        proposed_last_commit: Some(CommitInfo {
            votes: vec![],
            round: 0u16.into(),
        }),
        misbehavior: vec![],
    };

    let error = fixture
        .app
        .process_proposal(process_proposal.clone(), fixture.storage())
        .await
        .expect_err("expected transaction execution failure");

    assert_error_contains(
        &error,
        "failed to construct checked transactions in process proposal",
    );
}

#[tokio::test]
async fn app_end_block_validator_updates() {
    let mut fixture = Fixture::uninitialized(None).await;
    fixture
        .chain_initializer()
        .with_genesis_validators(vec![
            (ALICE.verification_key(), 100),
            (BOB.verification_key(), 1),
            (CAROL.verification_key(), 1),
        ])
        .init()
        .await;

    let proposer_address = [0u8; 20];
    let validator_updates = vec![
        ValidatorUpdate {
            power: 0,
            verification_key: ALICE.verification_key(),
            name: "Alice".parse().unwrap(),
        },
        ValidatorUpdate {
            power: 1,
            verification_key: BOB.verification_key(),
            name: "Bob".parse().unwrap(),
        },
        ValidatorUpdate {
            power: 100,
            verification_key: CAROL.verification_key(),
            name: "Carol".parse().unwrap(),
        },
        ValidatorUpdate {
            power: 100,
            verification_key: SUDO.verification_key(),
            name: "Sudo".parse().unwrap(),
        },
    ];

    fixture
        .state_mut()
        .put_block_validator_updates(ValidatorSet::new_from_updates(validator_updates.clone()))
        .unwrap();

    let resp = fixture.app.end_block(1, &proposer_address).await.unwrap();
    // we only assert length here as the ordering of the updates is not guaranteed
    // and validator::Update does not implement Ord
    assert_eq!(resp.validator_updates.len(), validator_updates.len());

    // Alice should be removed (power set to 0)
    // Bob should be unchanged
    // Carol's power should be updated
    // Sudo should be added
    let validator_set = fixture.state().pre_aspen_get_validator_set().await.unwrap();
    assert_eq!(validator_set.len(), 3);

    assert!(validator_set.get(&*ALICE_ADDRESS_BYTES).is_none());

    let bob = validator_set.get(&*BOB_ADDRESS_BYTES).unwrap();
    assert_eq!(bob.verification_key, BOB.verification_key());
    assert_eq!(bob.power, 1);

    let carol = validator_set.get(&*CAROL_ADDRESS_BYTES).unwrap();
    assert_eq!(carol.verification_key, CAROL.verification_key());
    assert_eq!(carol.power, 100);

    let sudo = validator_set.get(&*SUDO_ADDRESS_BYTES).unwrap();
    assert_eq!(sudo.verification_key, SUDO.verification_key());
    assert_eq!(sudo.power, 100);

    assert_eq!(
        fixture
            .state()
            .get_block_validator_updates()
            .await
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
#[expect(clippy::too_many_lines, reason = "it's a test")]
async fn app_proposal_fingerprint_triggers_update() {
    let mut fixture = Fixture::default_initialized().await;
    let height = fixture.block_height().await.increment();
    let bridge_address = astria_address(&[99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    fixture
        .bridge_initializer(bridge_address)
        .with_rollup_id(rollup_id)
        .init()
        .await;

    // Commit after `chain_init` should clear the fingerprint
    assert_eq!(*fixture.app.execution_state.data(), ExecutionState::Unset);

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

    let tx = fixture
        .checked_tx_builder()
        .with_action(lock_action)
        .with_action(rollup_data_submission)
        .with_signer(ALICE.clone())
        .build()
        .await;

    let expected_deposit = Deposit {
        bridge_address,
        rollup_id,
        amount,
        asset: nria().into(),
        destination_chain_address: "nootwashere".to_string(),
        source_transaction_id: *tx.id(),
        source_action_index: 0,
    };
    let deposits = HashMap::from_iter(vec![(rollup_id, vec![expected_deposit])]);

    let timestamp = Time::now();
    let raw_hash = [99u8; 32];
    let block_hash = Hash::Sha256(raw_hash);
    let txs_with_commitments = transactions_with_extended_commit_info_and_commitments(
        height,
        &[tx.clone()],
        Some(deposits.clone()),
    );

    // These two proposals match exactly, except for the proposer
    let prepare_proposal = PrepareProposal {
        max_tx_bytes: 1_000_000,
        height,
        time: timestamp,
        next_validators_hash: Hash::default(),
        proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
        txs: vec![],
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
        txs: txs_with_commitments,
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
    fixture
        .mempool()
        .insert(tx, 0, &dummy_balances(0, 0), dummy_tx_costs(0, 0, 0))
        .await
        .unwrap();
    fixture
        .app
        .prepare_proposal(prepare_proposal.clone(), fixture.storage())
        .await
        .unwrap();
    let ExecutionState::Prepared(cached_proposal) = fixture.app.execution_state.data().clone()
    else {
        panic!("should be in prepared state")
    };
    fixture
        .app
        .process_proposal(match_process_proposal.clone(), fixture.storage())
        .await
        .unwrap();
    let expected_state = ExecutionState::ExecutedBlock {
        cached_block_hash: raw_hash,
        cached_proposal: Some(cached_proposal),
    };
    assert_eq!(*fixture.app.execution_state.data(), expected_state);
    fixture
        .app
        .finalize_block(finalize_block.clone(), fixture.storage())
        .await
        .unwrap();
    assert_eq!(*fixture.app.execution_state.data(), expected_state);

    // Call PrepareProposal, then ProcessProposal where the proposals do not match
    // - validate the prepared proposal fingerprint
    // - after process proposal should have a ExecuteBlock fingerprint w/ no prepare fingerprint
    //   data
    // - finalize block should not change the fingerprint
    fixture
        .app
        .prepare_proposal(prepare_proposal.clone(), fixture.storage())
        .await
        .unwrap();
    fixture
        .app
        .process_proposal(non_match_process_proposal.clone(), fixture.storage())
        .await
        .unwrap();
    let expected_state = ExecutionState::ExecutedBlock {
        cached_block_hash: raw_hash,
        cached_proposal: None,
    };
    assert_eq!(*fixture.app.execution_state.data(), expected_state);
    fixture
        .app
        .finalize_block(finalize_block.clone(), fixture.storage())
        .await
        .unwrap();
    assert_eq!(*fixture.app.execution_state.data(), expected_state);

    // Cannot use fingerprint to jump from prepare proposal straight to finalize block
    // - the fingerprint at the end of finalize block should not have the prepare proposal data
    fixture
        .app
        .prepare_proposal(prepare_proposal.clone(), fixture.storage())
        .await
        .unwrap();
    fixture
        .app
        .finalize_block(finalize_block.clone(), fixture.storage())
        .await
        .unwrap();
    assert_eq!(*fixture.app.execution_state.data(), expected_state);

    // Calling update state for new round should reset key
    fixture.app.update_state_for_new_round(&fixture.storage());
    assert_eq!(*fixture.app.execution_state.data(), ExecutionState::Unset);
}

#[expect(clippy::too_many_lines, reason = "it's a test")]
#[tokio::test]
async fn app_oracle_price_update_events_in_finalize_block() {
    let mut fixture = Fixture::uninitialized(None).await;
    fixture
        .chain_initializer()
        .with_genesis_validators(vec![(ALICE.verification_key(), 100)])
        .init()
        .await;
    let height = fixture.run_until_aspen_applied().await;

    let currency_pair: CurrencyPair = "ETH/USD".parse().unwrap();
    let id = CurrencyPairId::new(0);
    let currency_pair_state = CurrencyPairState {
        price: None,
        nonce: CurrencyPairNonce::new(0),
        id,
    };
    fixture
        .state_mut()
        .put_currency_pair_state(currency_pair.clone(), currency_pair_state)
        .unwrap();
    fixture
        .app
        .prepare_commit(fixture.storage(), HashSet::new())
        .await
        .unwrap();
    fixture.app.commit(fixture.storage()).await.unwrap();

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
            address: *ALICE_ADDRESS_BYTES,
            power: 100u32.into(),
        },
        sig_info: BlockSignatureInfo::Flag(BlockIdFlag::Commit),
        vote_extension: extension_bytes.into(),
        extension_signature: Some(
            ALICE
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
    let finalize_block = fixture
        .app
        .finalize_block(finalize_block, fixture.storage())
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

#[tokio::test]
async fn all_event_attributes_should_be_indexed() {
    let mut fixture = Fixture::default_initialized().await;
    fixture.bridge_initializer(*BOB_ADDRESS).init().await;

    let value = 333_333;

    let transfer_action = Transfer {
        to: *BOB_ADDRESS,
        amount: value,
        asset: nria().into(),
        fee_asset: nria().into(),
    };
    let bridge_lock_action = BridgeLock {
        to: *BOB_ADDRESS,
        amount: 1,
        asset: nria().into(),
        fee_asset: nria().into(),
        destination_chain_address: "test_chain_address".to_string(),
    };
    let tx = fixture
        .checked_tx_builder()
        .with_action(transfer_action)
        .with_action(bridge_lock_action)
        .with_signer(ALICE.clone())
        .build()
        .await;

    let events = fixture.app.execute_transaction(tx).await.unwrap();

    events
        .iter()
        .flat_map(|event| &event.attributes)
        .for_each(|attribute| {
            assert!(
                attribute.index(),
                "attribute {} is not indexed",
                String::from_utf8_lossy(attribute.key_bytes()),
            );
        });
}
