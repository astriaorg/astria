use astria_core::primitive::v1::{
    asset::DEFAULT_NATIVE_ASSET_DENOM,
    Address,
    ADDRESS_LEN,
};
use cnidarium::Storage;
use ed25519_consensus::SigningKey;
use penumbra_ibc::params::IBCParameters;

use crate::{
    app::App,
    genesis::{
        Account,
        GenesisState,
    },
};

// attempts to decode the given hex string into an address.
pub(crate) fn address_from_hex_string(s: &str) -> Address {
    let bytes = hex::decode(s).unwrap();
    let arr: [u8; ADDRESS_LEN] = bytes.try_into().unwrap();
    Address::from_array(arr)
}

pub(crate) const ALICE_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
pub(crate) const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";
pub(crate) const CAROL_ADDRESS: &str = "60709e2d391864b732b4f0f51e387abb76743871";

pub(crate) fn get_alice_signing_key_and_address() -> (SigningKey, Address) {
    // this secret key corresponds to ALICE_ADDRESS
    let alice_secret_bytes: [u8; 32] =
        hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
            .unwrap()
            .try_into()
            .unwrap();
    let alice_signing_key = SigningKey::from(alice_secret_bytes);
    let alice = Address::from_verification_key(alice_signing_key.verification_key());
    (alice_signing_key, alice)
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

pub(crate) async fn initialize_app_with_storage(
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

pub(crate) async fn initialize_app(
    genesis_state: Option<GenesisState>,
    genesis_validators: Vec<tendermint::validator::Update>,
) -> App {
    let (app, _storage) = initialize_app_with_storage(genesis_state, genesis_validators).await;
    app
}
