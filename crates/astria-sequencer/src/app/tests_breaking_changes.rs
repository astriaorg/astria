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
    crypto::SigningKey,
    oracles::price_feed::{
        market_map::v2::Market,
        types::v2::CurrencyPair,
    },
    primitive::v1::RollupId,
    protocol::transaction::v1::action::{
        BridgeLock,
        BridgeSudoChange,
        BridgeUnlock,
        CurrencyPairsChange,
        FeeAssetChange,
        IbcRelayerChange,
        IbcSudoChange,
        InitBridgeAccount,
        MarketsChange,
        RollupDataSubmission,
        SudoAddressChange,
        Transfer,
        ValidatorUpdate,
    },
    sequencerblock::v1::block::Deposit,
};
use prost::bytes::Bytes;
use tendermint::{
    abci,
    abci::types::CommitInfo,
    block::Round,
    Hash,
    Time,
};

use crate::{
    authority::StateReadExt as _,
    bridge::StateWriteExt as _,
    test_utils::{
        astria_address,
        dummy_ticker,
        nria,
        transactions_with_extended_commit_info_and_commitments,
        Fixture,
        ALICE,
        ALICE_ADDRESS,
        BOB_ADDRESS,
        CAROL_ADDRESS,
        IBC_SUDO,
        IBC_SUDO_ADDRESS,
        SUDO,
        SUDO_ADDRESS,
        TEN_QUINTILLION,
    },
};

#[tokio::test]
async fn app_genesis_snapshot() {
    let (app, _storage) = Fixture::legacy_initialized().await.destructure();
    insta::assert_json_snapshot!("app_hash_at_genesis", hex::encode(app.app_hash.as_bytes()));
}

#[tokio::test]
async fn app_finalize_block_snapshot() {
    let mut fixture = Fixture::legacy_initialized().await;
    let height = fixture.run_until_blackburn_applied().await;

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

    // the state changes must be committed, as `finalize_block` will execute the
    // changes on the latest snapshot, not the app's `StateDelta`.
    fixture
        .app
        .prepare_commit(fixture.storage(), Vec::new())
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

    let timestamp = Time::unix_epoch();
    let block_hash = Hash::try_from([99u8; 32].to_vec()).unwrap();
    let finalize_block = abci::request::FinalizeBlock {
        hash: block_hash,
        height,
        time: timestamp,
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
        .finalize_block(finalize_block.clone(), fixture.storage())
        .await
        .unwrap();
    fixture.app.commit(fixture.storage()).await.unwrap();
    insta::assert_json_snapshot!(
        "app_hash_finalize_block",
        hex::encode(fixture.app.app_hash.as_bytes())
    );
}

// Note: this tests every action except for `Ics20Withdrawal` and `IbcRelay`.
//
// If new actions are added to the app, they must be added to this test,
// and the respective PR must be marked as breaking.
#[expect(clippy::too_many_lines, reason = "it's a test")]
#[tokio::test]
async fn app_legacy_execute_transactions_with_every_action_snapshot() {
    use rand::SeedableRng as _;

    let mut fixture = Fixture::uninitialized(None).await;
    fixture
        .legacy_chain_initializer()
        .with_genesis_accounts(vec![
            (*ALICE_ADDRESS, TEN_QUINTILLION),
            (*BOB_ADDRESS, TEN_QUINTILLION),
            (*CAROL_ADDRESS, TEN_QUINTILLION),
            (*IBC_SUDO_ADDRESS, 1_000_000_000),
            (*SUDO_ADDRESS, 1_000_000_000),
        ])
        .with_authority_sudo_address(*ALICE_ADDRESS)
        .with_ibc_sudo_address(*ALICE_ADDRESS)
        .init()
        .await;

    let height = fixture.run_until_blackburn_applied().await;

    let bridge = IBC_SUDO.clone();
    let bridge_withdrawer = SUDO.clone();
    let bridge_address = *IBC_SUDO_ADDRESS;
    let bob_address = *BOB_ADDRESS;
    let carol_address = *CAROL_ADDRESS;
    let bridge_withdrawer_address = *SUDO_ADDRESS;

    let verification_key = {
        let rng = rand_chacha::ChaChaRng::seed_from_u64(1);
        let signing_key = SigningKey::new(rng);
        signing_key.verification_key()
    };

    // setup for ValidatorUpdate action
    let update = ValidatorUpdate {
        name: "test_validator".parse().unwrap(),
        power: 100,
        verification_key,
    };

    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");

    let tx_bundleable_general = fixture
        .checked_tx_builder()
        .with_action(Transfer {
            to: bob_address,
            amount: 333_333,
            asset: nria().into(),
            fee_asset: nria().into(),
        })
        .with_action(RollupDataSubmission {
            rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
            data: Bytes::from_static(b"hello world"),
            fee_asset: nria().into(),
        })
        .with_action(update.clone())
        .with_signer(ALICE.clone())
        .build()
        .await;

    let tx_bundleable_sudo = fixture
        .checked_tx_builder()
        .with_action(IbcRelayerChange::Addition(bob_address))
        .with_action(IbcRelayerChange::Addition(carol_address))
        .with_action(IbcRelayerChange::Removal(bob_address))
        .with_action(FeeAssetChange::Addition("test-0".parse().unwrap()))
        .with_action(FeeAssetChange::Addition("test-1".parse().unwrap()))
        .with_action(FeeAssetChange::Removal("test-0".parse().unwrap()))
        .with_action(CurrencyPairsChange::Addition(
            std::iter::once("TIA/USD".parse::<CurrencyPair>().unwrap()).collect(),
        ))
        .with_action(CurrencyPairsChange::Removal(
            std::iter::once("ETH/USD".parse::<CurrencyPair>().unwrap()).collect(),
        ))
        .with_action(MarketsChange::Creation(vec![Market {
            ticker: dummy_ticker("testAssetOne/testAssetTwo", "create market"),
            provider_configs: vec![],
        }]))
        .with_action(MarketsChange::Update(vec![Market {
            ticker: dummy_ticker("testAssetOne/testAssetTwo", "update market"),
            provider_configs: vec![],
        }]))
        .with_action(MarketsChange::Removal(vec![Market {
            ticker: dummy_ticker("testAssetOne/testAssetTwo", "remove market"),
            provider_configs: vec![],
        }]))
        .with_nonce(1)
        .with_signer(ALICE.clone())
        .build()
        .await;

    let tx_sudo_ibc = fixture
        .checked_tx_builder()
        .with_action(IbcSudoChange {
            new_address: bob_address,
        })
        .with_nonce(2)
        .with_signer(ALICE.clone())
        .build()
        .await;

    let tx_sudo = fixture
        .checked_tx_builder()
        .with_action(SudoAddressChange {
            new_address: bob_address,
        })
        .with_nonce(3)
        .with_signer(ALICE.clone())
        .build()
        .await;

    fixture
        .app
        .execute_transaction(tx_bundleable_general)
        .await
        .unwrap();
    fixture
        .app
        .execute_transaction(tx_bundleable_sudo)
        .await
        .unwrap();
    fixture.app.execute_transaction(tx_sudo_ibc).await.unwrap();
    fixture.app.execute_transaction(tx_sudo).await.unwrap();

    let tx = fixture
        .checked_tx_builder()
        .with_action(InitBridgeAccount {
            rollup_id,
            asset: nria().into(),
            fee_asset: nria().into(),
            sudo_address: None,
            withdrawer_address: Some(bridge_withdrawer_address),
        })
        .with_signer(bridge.clone())
        .build()
        .await;
    fixture.app.execute_transaction(tx).await.unwrap();

    let tx_bridge_bundleable = fixture
        .checked_tx_builder()
        .with_action(BridgeLock {
            to: bridge_address,
            amount: 100,
            asset: nria().into(),
            fee_asset: nria().into(),
            destination_chain_address: "nootwashere".to_string(),
        })
        .with_action(BridgeUnlock {
            to: bob_address,
            amount: 10,
            fee_asset: nria().into(),
            memo: String::new(),
            bridge_address,
            rollup_block_number: 1,
            rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
        })
        .with_signer(bridge_withdrawer.clone())
        .build()
        .await;
    fixture
        .app
        .execute_transaction(tx_bridge_bundleable)
        .await
        .unwrap();

    let tx_bridge = fixture
        .checked_tx_builder()
        .with_action(BridgeSudoChange {
            bridge_address,
            new_sudo_address: Some(bob_address),
            new_withdrawer_address: Some(bob_address),
            fee_asset: nria().into(),
            disable_deposits: false,
        })
        .with_nonce(1)
        .with_signer(bridge.clone())
        .build()
        .await;
    fixture.app.execute_transaction(tx_bridge).await.unwrap();

    let sudo_address = fixture.app.state.get_sudo_address().await.unwrap();
    fixture
        .app
        .end_block(height.value(), &sudo_address)
        .await
        .unwrap();

    fixture
        .app
        .prepare_commit(fixture.storage(), Vec::new())
        .await
        .unwrap();
    fixture.app.commit(fixture.storage()).await.unwrap();

    insta::assert_json_snapshot!(
        "app_hash_execute_every_action",
        hex::encode(fixture.app.app_hash.as_bytes())
    );
}
