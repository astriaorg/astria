use std::sync::Arc;

use astria_core::{
    crypto::SigningKey,
    primitive::v1::{
        Address,
        RollupId,
        ADDRESS_LEN,
    },
    protocol::transaction::v1alpha1::{
        action::{
            SequenceAction,
            ValidatorUpdate,
        },
        SignedTransaction,
        TransactionParams,
        UnsignedTransaction,
    },
    sequencer::{
        Account,
        AddressPrefixes,
        Fees,
        GenesisState,
        UncheckedGenesisState,
    },
};
use cnidarium::Storage;
use penumbra_ibc::params::IBCParameters;

use crate::{
    app::App,
    mempool::Mempool,
    metrics::Metrics,
};

// attempts to decode the given hex string into an address.
pub(crate) fn address_from_hex_string(s: &str) -> Address {
    let bytes = hex::decode(s).unwrap();
    let arr: [u8; ADDRESS_LEN] = bytes.try_into().unwrap();
    crate::address::base_prefixed(arr)
}

pub(crate) const ALICE_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
pub(crate) const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";
pub(crate) const CAROL_ADDRESS: &str = "60709e2d391864b732b4f0f51e387abb76743871";
pub(crate) const JUDY_ADDRESS: &str = "bc5b91da07778eeaf622d0dcf4d7b4233525998d";
pub(crate) const TED_ADDRESS: &str = "4c4f91d8a918357ab5f6f19c1e179968fc39bb44";

pub(crate) fn get_alice_signing_key_and_address() -> (SigningKey, Address) {
    // this secret key corresponds to ALICE_ADDRESS
    let alice_secret_bytes: [u8; 32] =
        hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
            .unwrap()
            .try_into()
            .unwrap();
    let alice_signing_key = SigningKey::from(alice_secret_bytes);
    let alice = crate::address::base_prefixed(alice_signing_key.verification_key().address_bytes());
    (alice_signing_key, alice)
}

pub(crate) fn get_bridge_signing_key_and_address() -> (SigningKey, Address) {
    let bridge_secret_bytes: [u8; 32] =
        hex::decode("db4982e01f3eba9e74ac35422fcd49aa2b47c3c535345c7e7da5220fe3a0ce79")
            .unwrap()
            .try_into()
            .unwrap();
    let bridge_signing_key = SigningKey::from(bridge_secret_bytes);
    let bridge =
        crate::address::base_prefixed(bridge_signing_key.verification_key().address_bytes());
    (bridge_signing_key, bridge)
}

pub(crate) fn default_genesis_accounts() -> Vec<Account> {
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

pub(crate) fn default_fees() -> Fees {
    Fees {
        transfer_base_fee: 12,
        sequence_base_fee: 32,
        sequence_byte_cost_multiplier: 1,
        init_bridge_account_base_fee: 48,
        bridge_lock_byte_cost_multiplier: 1,
        bridge_sudo_change_fee: 24,
        ics20_withdrawal_base_fee: 24,
    }
}

pub(crate) fn unchecked_genesis_state() -> UncheckedGenesisState {
    UncheckedGenesisState {
        accounts: default_genesis_accounts(),
        address_prefixes: AddressPrefixes {
            base: crate::address::get_base_prefix().to_string(),
        },
        authority_sudo_address: address_from_hex_string(JUDY_ADDRESS),
        ibc_sudo_address: address_from_hex_string(TED_ADDRESS),
        ibc_relayer_addresses: vec![],
        native_asset_base_denomination: "nria".to_string(),
        ibc_params: IBCParameters::default(),
        allowed_fee_assets: vec!["nria".parse().unwrap()],
        fees: default_fees(),
    }
}

pub(crate) fn genesis_state() -> GenesisState {
    unchecked_genesis_state().try_into().unwrap()
}

pub(crate) async fn initialize_app_with_storage(
    genesis_state: Option<GenesisState>,
    genesis_validators: Vec<ValidatorUpdate>,
) -> (App, Storage) {
    let storage = cnidarium::TempStorage::new()
        .await
        .expect("failed to create temp storage backing chain state");
    let snapshot = storage.latest_snapshot();
    let mempool = Mempool::new();
    let metrics = Box::leak(Box::new(Metrics::new()));
    let mut app = App::new(snapshot, mempool, metrics).await.unwrap();

    let genesis_state = genesis_state.unwrap_or_else(self::genesis_state);

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

pub(crate) async fn initialize_app(
    genesis_state: Option<GenesisState>,
    genesis_validators: Vec<ValidatorUpdate>,
) -> App {
    let (app, _storage) = initialize_app_with_storage(genesis_state, genesis_validators).await;
    app
}

pub(crate) fn get_mock_tx(nonce: u32) -> Arc<SignedTransaction> {
    let (alice_signing_key, _) = get_alice_signing_key_and_address();
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(nonce)
            .chain_id("test")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes([0; 32]),
                data: vec![0x99],
                fee_asset: "astria".parse().unwrap(),
            }
            .into(),
        ],
    };

    Arc::new(tx.into_signed(&alice_signing_key))
}
