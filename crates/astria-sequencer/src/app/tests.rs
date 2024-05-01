use std::{
    collections::{
        HashMap,
        VecDeque,
    },
    sync::Arc,
};

use anyhow::{
    anyhow,
    ensure,
    Context,
};
#[cfg(feature = "mint")]
use astria_core::protocol::transaction::v1alpha1::action::MintAction;
use astria_core::{
    generated::protocol::transaction::v1alpha1 as raw,
    primitive::v1::{
        asset,
        asset::DEFAULT_NATIVE_ASSET_DENOM,
        Address,
        RollupId,
    },
    protocol::{
        abci::AbciErrorCode,
        transaction::v1alpha1::{
            action::{
                BridgeLockAction,
                IbcRelayerChangeAction,
                SequenceAction,
                SudoAddressChangeAction,
                TransferAction,
            },
            Action,
            SignedTransaction,
            TransactionParams,
            UnsignedTransaction,
        },
    },
    sequencerblock::v1alpha1::block::{
        Deposit,
        SequencerBlock,
    },
};
use cnidarium::{
    ArcStateDeltaExt,
    Snapshot,
    StagedWriteBatch,
    StateDelta,
    Storage,
};
use ed25519_consensus::SigningKey;
use penumbra_ibc::params::IBCParameters;
use prost::Message as _;
use sha2::{
    Digest as _,
    Sha256,
};
use telemetry::display::json;
use tendermint::{
    abci::{
        self,
        request::PrepareProposal,
        types::{
            CommitInfo,
            ExecTxResult,
        },
        Event,
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
    accounts::{
        state_ext::{
            StateReadExt as _,
        },
    },
    api_state_ext::StateWriteExt as _,
    app::App,
    app::test_utils::*,
    asset::get_native_asset,
    authority::{
        component::{
            AuthorityComponent,
            AuthorityComponentAppState,
        },
        state_ext::{
            StateReadExt as _,
            StateWriteExt as _,
            ValidatorSet,
        },
    },
    bridge::{
        component::BridgeComponent,
        state_ext::{
            StateReadExt as _,
            StateWriteExt,
        },
    },
    component::Component as _,
    genesis::{
        Account,
        GenesisState,
    },
    ibc::{
        component::IbcComponent,
        state_ext::StateReadExt as _,
    },
    metrics_init,
    proposal::{
        block_size_constraints::BlockSizeConstraints,
        commitment::{
            generate_rollup_datas_commitment,
            GeneratedCommitments,
        },
    },
    sequence::{
        calculate_fee_from_state,
        component::SequenceComponent,
    },
    state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::{
        self,
        InvalidChainId,
        InvalidNonce,
    },
};

fn default_genesis_accounts() -> Vec<Account> {
    vec![
        Account {
            address: address_from_hex_string(ALICE_ADDRESS),
            balance: 10u128.pow(19),
        },
        Account {
            address: address_from_hex_string(BOB_ADDRESS),
            balance: 10u128.pow(19),
        },
        Account {
            address: address_from_hex_string(CAROL_ADDRESS),
            balance: 10u128.pow(19),
        },
    ]
}

fn default_header() -> Header {
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

async fn initialize_app_with_storage(
    genesis_state: Option<GenesisState>,
    genesis_validators: Vec<tendermint::validator::Update>,
) -> (App, Storage) {
    let storage = cnidarium::TempStorage::new()
        .await
        .expect("failed to create temp storage backing chain state");
    let snapshot = storage.latest_snapshot();
    let mut app = App::new(snapshot).await.unwrap();

    let genesis_state = genesis_state.unwrap_or_else(|| GenesisState {
        accounts: default_genesis_accounts(),
        authority_sudo_address: Address::from([0; 20]),
        ibc_sudo_address: Address::from([0; 20]),
        ibc_relayer_addresses: vec![],
        native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
        ibc_params: IBCParameters::default(),
        allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
    });

    app.init_chain(
        storage.clone(),
        genesis_state,
        genesis_validators,
        "test".to_string(),
    )
    .await
    .unwrap();
    app.commit(storage.clone()).await;

    (app, storage.clone())
}

async fn initialize_app(
    genesis_state: Option<GenesisState>,
    genesis_validators: Vec<tendermint::validator::Update>,
) -> App {
    let (app, _storage) = initialize_app_with_storage(genesis_state, genesis_validators).await;

    app
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
        header: default_header(),
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
async fn app_execute_transaction_transfer() {
    let mut app = initialize_app(None, vec![]).await;

    // transfer funds from Alice to Bob
    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
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

    let native_asset = get_native_asset().id();
    assert_eq!(
        app.state
            .get_account_balance(bob_address, native_asset)
            .await
            .unwrap(),
        value + 10u128.pow(19)
    );
    let transfer_fee = app.state.get_transfer_base_fee().await.unwrap();
    assert_eq!(
        app.state
            .get_account_balance(alice_address, native_asset)
            .await
            .unwrap(),
        10u128.pow(19) - (value + transfer_fee),
    );
    assert_eq!(app.state.get_account_nonce(bob_address).await.unwrap(), 0);
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
}

#[tokio::test]
async fn app_execute_transaction_transfer_not_native_token() {
    use crate::accounts::state_ext::StateWriteExt as _;

    let mut app = initialize_app(None, vec![]).await;

    // create some asset to be transferred and update Alice's balance of it
    let asset = asset::Id::from_denom("test");
    let value = 333_333;
    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx
        .put_account_balance(alice_address, asset, value)
        .unwrap();
    app.apply(state_tx);

    // transfer funds from Alice to Bob; use native token for fee payment
    let bob_address = address_from_hex_string(BOB_ADDRESS);
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            TransferAction {
                to: bob_address,
                amount: value,
                asset_id: asset,
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();

    let native_asset = get_native_asset().id();
    assert_eq!(
        app.state
            .get_account_balance(bob_address, native_asset)
            .await
            .unwrap(),
        10u128.pow(19), // genesis balance
    );
    assert_eq!(
        app.state
            .get_account_balance(bob_address, asset)
            .await
            .unwrap(),
        value, // transferred amount
    );

    let transfer_fee = app.state.get_transfer_base_fee().await.unwrap();
    assert_eq!(
        app.state
            .get_account_balance(alice_address, native_asset)
            .await
            .unwrap(),
        10u128.pow(19) - transfer_fee, // genesis balance - fee
    );
    assert_eq!(
        app.state
            .get_account_balance(alice_address, asset)
            .await
            .unwrap(),
        0, // 0 since all funds of `asset` were transferred
    );

    assert_eq!(app.state.get_account_nonce(bob_address).await.unwrap(), 0);
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
}

#[tokio::test]
async fn app_execute_transaction_transfer_balance_too_low_for_fee() {
    use rand::rngs::OsRng;

    let mut app = initialize_app(None, vec![]).await;

    // create a new key; will have 0 balance
    let keypair = SigningKey::new(OsRng);
    let bob = address_from_hex_string(BOB_ADDRESS);

    // 0-value transfer; only fee is deducted from sender
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            TransferAction {
                to: bob,
                amount: 0,
                asset_id: get_native_asset().id(),
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&keypair);
    let res = app
        .execute_transaction(signed_tx)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(res.contains("insufficient funds"));
}

#[tokio::test]
async fn app_stateful_check_fails_insufficient_total_balance() {
    use rand::rngs::OsRng;
    let mut app = initialize_app(None, vec![]).await;

    let (alice_signing_key, _) = get_alice_signing_key_and_address();

    // create a new key; will have 0 balance
    let keypair = SigningKey::new(OsRng);
    let keypair_address = Address::from_verification_key(keypair.verification_key());

    // figure out needed fee for a single transfer
    let data = b"hello world".to_vec();
    let fee = calculate_fee_from_state(&data, &app.state.clone())
        .await
        .unwrap();

    // transfer just enough to cover single sequence fee with data
    let signed_tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            TransferAction {
                to: keypair_address,
                amount: fee,
                asset_id: get_native_asset().id(),
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    }
    .into_signed(&alice_signing_key);

    // make transfer
    app.execute_transaction(signed_tx).await.unwrap();

    // build double transfer exceeding balance
    let signed_tx_fail = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data: data.clone(),
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data: data.clone(),
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    }
    .into_signed(&keypair);

    // try double, see fails stateful check
    let res = transaction::check_stateful(&signed_tx_fail, &app.state)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(res.contains("insufficient funds for asset"));

    // build single transfer to see passes
    let signed_tx_pass = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    }
    .into_signed(&keypair);

    transaction::check_stateful(&signed_tx_pass, &app.state)
        .await
        .expect("stateful check should pass since we transferred enough to cover fee");
}

#[tokio::test]
async fn app_execute_transaction_sequence() {
    use crate::sequence::state_ext::StateWriteExt as _;

    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx.put_sequence_action_base_fee(0);
    state_tx.put_sequence_action_byte_cost_multiplier(1);
    app.apply(state_tx);

    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
    let data = b"hello world".to_vec();
    let fee = calculate_fee_from_state(&data, &app.state).await.unwrap();

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    assert_eq!(
        app.state
            .get_account_balance(alice_address, get_native_asset().id())
            .await
            .unwrap(),
        10u128.pow(19) - fee,
    );
}

#[tokio::test]
async fn app_execute_transaction_invalid_fee_asset() {
    let mut app = initialize_app(None, vec![]).await;

    let (alice_signing_key, _) = get_alice_signing_key_and_address();
    let data = b"hello world".to_vec();

    let fee_asset_id = asset::Id::from_denom("test");

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset_id,
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[tokio::test]
async fn app_execute_transaction_validator_update() {
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
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    let validator_updates = app.state.get_validator_updates().await.unwrap();
    assert_eq!(validator_updates.len(), 1);
    assert_eq!(validator_updates.get(&pub_key.into()).unwrap(), &update);
}

#[tokio::test]
async fn app_execute_transaction_ibc_relayer_change_addition() {
    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

    let genesis_state = GenesisState {
        accounts: default_genesis_accounts(),
        authority_sudo_address: alice_address,
        ibc_sudo_address: alice_address,
        ibc_relayer_addresses: vec![],
        native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
        allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        ibc_params: IBCParameters::default(),
    };
    let mut app = initialize_app(Some(genesis_state), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![IbcRelayerChangeAction::Addition(alice_address).into()],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
    assert!(app.state.is_ibc_relayer(&alice_address).await.unwrap());
}

#[tokio::test]
async fn app_execute_transaction_ibc_relayer_change_deletion() {
    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

    let genesis_state = GenesisState {
        accounts: default_genesis_accounts(),
        authority_sudo_address: alice_address,
        ibc_sudo_address: alice_address,
        ibc_relayer_addresses: vec![alice_address],
        native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
        allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        ibc_params: IBCParameters::default(),
    };
    let mut app = initialize_app(Some(genesis_state), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![IbcRelayerChangeAction::Removal(alice_address).into()],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
    assert!(!app.state.is_ibc_relayer(&alice_address).await.unwrap());
}

#[tokio::test]
async fn app_execute_transaction_ibc_relayer_change_invalid() {
    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

    let genesis_state = GenesisState {
        accounts: default_genesis_accounts(),
        authority_sudo_address: alice_address,
        ibc_sudo_address: Address::from([0; 20]),
        ibc_relayer_addresses: vec![alice_address],
        native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
        allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        ibc_params: IBCParameters::default(),
    };
    let mut app = initialize_app(Some(genesis_state), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![IbcRelayerChangeAction::Removal(alice_address).into()],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[tokio::test]
async fn app_execute_transaction_sudo_address_change() {
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

    let new_address = address_from_hex_string(BOB_ADDRESS);

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![Action::SudoAddressChange(SudoAddressChangeAction {
            new_address,
        })],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    let sudo_address = app.state.get_sudo_address().await.unwrap();
    assert_eq!(sudo_address, new_address);
}

#[tokio::test]
async fn app_execute_transaction_sudo_address_change_error() {
    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
    let sudo_address = address_from_hex_string(CAROL_ADDRESS);

    let genesis_state = GenesisState {
        accounts: default_genesis_accounts(),
        authority_sudo_address: sudo_address,
        ibc_sudo_address: [0u8; 20].into(),
        ibc_relayer_addresses: vec![],
        native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
        ibc_params: IBCParameters::default(),
        allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
    };
    let mut app = initialize_app(Some(genesis_state), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![Action::SudoAddressChange(SudoAddressChangeAction {
            new_address: alice_address,
        })],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    let res = app
        .execute_transaction(signed_tx)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(res.contains("signer is not the sudo key"));
}

#[tokio::test]
async fn app_execute_transaction_fee_asset_change_addition() {
    use astria_core::protocol::transaction::v1alpha1::action::FeeAssetChangeAction;

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

    let new_asset = asset::Id::from_denom("test");

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![Action::FeeAssetChange(FeeAssetChangeAction::Addition(
            new_asset,
        ))],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    assert!(app.state.is_allowed_fee_asset(new_asset).await.unwrap());
}

#[tokio::test]
async fn app_execute_transaction_fee_asset_change_removal() {
    use astria_core::protocol::transaction::v1alpha1::action::FeeAssetChangeAction;

    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
    let test_asset = asset::Denom::from_base_denom("test");

    let genesis_state = GenesisState {
        accounts: default_genesis_accounts(),
        authority_sudo_address: alice_address,
        ibc_sudo_address: alice_address,
        ibc_relayer_addresses: vec![],
        native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
        ibc_params: IBCParameters::default(),
        allowed_fee_assets: vec![
            DEFAULT_NATIVE_ASSET_DENOM.to_owned().into(),
            test_asset.clone(),
        ],
    };
    let mut app = initialize_app(Some(genesis_state), vec![]).await;

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![Action::FeeAssetChange(FeeAssetChangeAction::Removal(
            test_asset.id(),
        ))],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);

    assert!(
        !app.state
            .is_allowed_fee_asset(test_asset.id())
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn app_execute_transaction_fee_asset_change_invalid() {
    use astria_core::protocol::transaction::v1alpha1::action::FeeAssetChangeAction;

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

    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![Action::FeeAssetChange(FeeAssetChangeAction::Removal(
            get_native_asset().id(),
        ))],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    let res = app
        .execute_transaction(signed_tx)
        .await
        .unwrap_err()
        .root_cause()
        .to_string();
    assert!(res.contains("cannot remove last allowed fee asset"));
}

#[tokio::test]
async fn app_execute_transaction_init_bridge_account_ok() {
    use astria_core::protocol::transaction::v1alpha1::action::InitBridgeAccountAction;

    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
    let mut app = initialize_app(None, vec![]).await;
    let mut state_tx = StateDelta::new(app.state.clone());
    let fee = 12; // arbitrary
    state_tx.put_init_bridge_account_base_fee(fee);
    app.apply(state_tx);

    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let asset_id = get_native_asset().id();
    let action = InitBridgeAccountAction {
        rollup_id,
        asset_id,
        fee_asset_id: asset_id,
    };
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![action.into()],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);

    let before_balance = app
        .state
        .get_account_balance(alice_address, asset_id)
        .await
        .unwrap();
    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
    assert_eq!(
        app.state
            .get_bridge_account_rollup_id(&alice_address)
            .await
            .unwrap()
            .unwrap(),
        rollup_id
    );
    assert_eq!(
        app.state
            .get_bridge_account_asset_ids(&alice_address)
            .await
            .unwrap(),
        asset_id
    );
    assert_eq!(
        app.state
            .get_account_balance(alice_address, asset_id)
            .await
            .unwrap(),
        before_balance - fee,
    );
}

#[tokio::test]
async fn app_execute_transaction_init_bridge_account_account_already_registered() {
    use astria_core::protocol::transaction::v1alpha1::action::InitBridgeAccountAction;

    let (alice_signing_key, _) = get_alice_signing_key_and_address();
    let mut app = initialize_app(None, vec![]).await;

    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let asset_id = get_native_asset().id();
    let action = InitBridgeAccountAction {
        rollup_id,
        asset_id,
        fee_asset_id: asset_id,
    };
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![action.into()],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();

    let action = InitBridgeAccountAction {
        rollup_id,
        asset_id,
        fee_asset_id: asset_id,
    };
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 1,
            chain_id: "test".to_string(),
        },
        actions: vec![action.into()],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[tokio::test]
async fn app_execute_transaction_bridge_lock_action_ok() {
    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();
    let mut app = initialize_app(None, vec![]).await;

    let bridge_address = Address::from([99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let asset_id = get_native_asset().id();

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
    state_tx
        .put_bridge_account_asset_id(&bridge_address, &asset_id)
        .unwrap();
    app.apply(state_tx);

    let amount = 100;
    let action = BridgeLockAction {
        to: bridge_address,
        amount,
        asset_id,
        fee_asset_id: asset_id,
        destination_chain_address: "nootwashere".to_string(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![action.into()],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);

    let alice_before_balance = app
        .state
        .get_account_balance(alice_address, asset_id)
        .await
        .unwrap();
    let bridge_before_balance = app
        .state
        .get_account_balance(bridge_address, asset_id)
        .await
        .unwrap();

    app.execute_transaction(signed_tx).await.unwrap();
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
    let transfer_fee = app.state.get_transfer_base_fee().await.unwrap();
    let expected_deposit = Deposit::new(
        bridge_address,
        rollup_id,
        amount,
        asset_id,
        "nootwashere".to_string(),
    );

    let fee = transfer_fee
        + app
            .state
            .get_bridge_lock_byte_cost_multiplier()
            .await
            .unwrap()
            * crate::bridge::get_deposit_byte_len(&expected_deposit);
    assert_eq!(
        app.state
            .get_account_balance(alice_address, asset_id)
            .await
            .unwrap(),
        alice_before_balance - (amount + fee)
    );
    assert_eq!(
        app.state
            .get_account_balance(bridge_address, asset_id)
            .await
            .unwrap(),
        bridge_before_balance + amount
    );

    let deposits = app.state.get_deposit_events(&rollup_id).await.unwrap();
    assert_eq!(deposits.len(), 1);
    assert_eq!(deposits[0], expected_deposit);
}

#[tokio::test]
async fn app_execute_transaction_bridge_lock_action_invalid_for_eoa() {
    use astria_core::protocol::transaction::v1alpha1::action::BridgeLockAction;

    let (alice_signing_key, _) = get_alice_signing_key_and_address();
    let mut app = initialize_app(None, vec![]).await;

    // don't actually register this address as a bridge address
    let bridge_address = Address::from([99; 20]);
    let asset_id = get_native_asset().id();

    let amount = 100;
    let action = BridgeLockAction {
        to: bridge_address,
        amount,
        asset_id,
        fee_asset_id: asset_id,
        destination_chain_address: "nootwashere".to_string(),
    };
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![action.into()],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[tokio::test]
async fn app_execute_transaction_transfer_invalid_to_bridge_account() {
    let (alice_signing_key, _) = get_alice_signing_key_and_address();
    let mut app = initialize_app(None, vec![]).await;

    let bridge_address = Address::from([99; 20]);
    let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
    let asset_id = get_native_asset().id();

    let mut state_tx = StateDelta::new(app.state.clone());
    state_tx.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
    state_tx
        .put_bridge_account_asset_id(&bridge_address, &asset_id)
        .unwrap();
    app.apply(state_tx);

    let amount = 100;
    let action = TransferAction {
        to: bridge_address,
        amount,
        asset_id,
        fee_asset_id: asset_id,
    };
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![action.into()],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    assert!(app.execute_transaction(signed_tx).await.is_err());
}

#[cfg(feature = "mint")]
#[tokio::test]
async fn app_execute_transaction_mint() {
    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

    let genesis_state = GenesisState {
        accounts: default_genesis_accounts(),
        authority_sudo_address: alice_address,
        ibc_sudo_address: [0u8; 20].into(),
        ibc_relayer_addresses: vec![],
        native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
        ibc_params: IBCParameters::default(),
        allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
    };
    let mut app = initialize_app(Some(genesis_state), vec![]).await;

    let bob_address = address_from_hex_string(BOB_ADDRESS);
    let value = 333_333;
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "test".to_string(),
        },
        actions: vec![
            MintAction {
                to: bob_address,
                amount: value,
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    app.execute_transaction(signed_tx).await.unwrap();

    assert_eq!(
        app.state
            .get_account_balance(bob_address, get_native_asset().id())
            .await
            .unwrap(),
        value + 10u128.pow(19)
    );
    assert_eq!(app.state.get_account_nonce(bob_address).await.unwrap(), 0);
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 1);
}

#[tokio::test]
async fn app_end_block_validator_updates() {
    use tendermint::validator;

    let pubkey_a = tendermint::public_key::PublicKey::from_raw_ed25519(&[1; 32]).unwrap();
    let pubkey_b = tendermint::public_key::PublicKey::from_raw_ed25519(&[2; 32]).unwrap();
    let pubkey_c = tendermint::public_key::PublicKey::from_raw_ed25519(&[3; 32]).unwrap();

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

    let mut app = initialize_app(None, initial_validator_set).await;
    let proposer_address = Address::try_from_slice([0u8; 20].as_ref()).unwrap();

    let validator_updates = vec![
        validator::Update {
            pub_key: pubkey_a,
            power: 0u32.into(),
        },
        validator::Update {
            pub_key: pubkey_b,
            power: 100u32.into(),
        },
        validator::Update {
            pub_key: pubkey_c,
            power: 100u32.into(),
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
    let validator_b = validator_set.get(&pubkey_b.into()).unwrap();
    assert_eq!(validator_b.pub_key, pubkey_b);
    assert_eq!(validator_b.power, 100u32.into());
    let validator_c = validator_set.get(&pubkey_c.into()).unwrap();
    assert_eq!(validator_c.pub_key, pubkey_c);
    assert_eq!(validator_c.power, 100u32.into());
    assert_eq!(app.state.get_validator_updates().await.unwrap().len(), 0);
}

#[tokio::test]
async fn app_execute_transaction_invalid_nonce() {
    let mut app = initialize_app(None, vec![]).await;

    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

    // create tx with invalid nonce 1
    let data = b"hello world".to_vec();
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 1,
            chain_id: "test".to_string(),
        },
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    let response = app.execute_transaction(signed_tx).await;

    // check that tx was not executed by checking nonce and balance are unchanged
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 0);
    assert_eq!(
        app.state
            .get_account_balance(alice_address, get_native_asset().id())
            .await
            .unwrap(),
        10u128.pow(19),
    );

    assert_eq!(
        response
            .unwrap_err()
            .downcast_ref::<InvalidNonce>()
            .map(|nonce_err| nonce_err.0)
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn app_deliver_tx_invalid_chain_id() {
    let mut app = initialize_app(None, vec![]).await;

    let (alice_signing_key, alice_address) = get_alice_signing_key_and_address();

    // create tx with invalid nonce 1
    let data = b"hello world".to_vec();
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce: 0,
            chain_id: "wrong-chain".to_string(),
        },
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data,
                fee_asset_id: get_native_asset().id(),
            }
            .into(),
        ],
    };

    let signed_tx = tx.into_signed(&alice_signing_key);
    let response = app.execute_transaction(signed_tx).await;

    // check that tx was not executed by checking nonce and balance are unchanged
    assert_eq!(app.state.get_account_nonce(alice_address).await.unwrap(), 0);
    assert_eq!(
        app.state
            .get_account_balance(alice_address, get_native_asset().id())
            .await
            .unwrap(),
        10u128.pow(19),
    );

    assert_eq!(
        response
            .unwrap_err()
            .downcast_ref::<InvalidChainId>()
            .map(|chain_id_err| &chain_id_err.0)
            .unwrap(),
        "wrong-chain"
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
