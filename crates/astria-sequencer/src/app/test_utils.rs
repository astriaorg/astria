use astria_core::{
    primitive::v1::{
        asset::DEFAULT_NATIVE_ASSET_DENOM,
        Address,
        RollupId,
        ADDRESS_LEN,
    },
    protocol::transaction::v1alpha1::{
        action::SequenceAction,
        SignedTransaction,
        TransactionParams,
        UnsignedTransaction,
    },
};
use cnidarium::Storage;
use ed25519_consensus::SigningKey;
use penumbra_ibc::params::IBCParameters;

use crate::{
    app::App,
    genesis::{
        self,
        Account,
        GenesisState,
    },
    mempool::Mempool,
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

pub(crate) fn default_fees() -> genesis::Fees {
    genesis::Fees {
        transfer_base_fee: 12,
        sequence_base_fee: 32,
        sequence_byte_cost_multiplier: 1,
        init_bridge_account_base_fee: 48,
        bridge_lock_byte_cost_multiplier: 1,
        ics20_withdrawal_base_fee: 24,
    }
}

pub(crate) async fn initialize_app_with_storage(
    genesis_state: Option<GenesisState>,
    genesis_validators: Vec<tendermint::validator::Update>,
) -> (App, Storage) {
    let storage = cnidarium::TempStorage::new()
        .await
        .expect("failed to create temp storage backing chain state");
    let snapshot = storage.latest_snapshot();
    let mempool = Mempool::new();
    let mut app = App::new(snapshot, mempool).await.unwrap();

    let genesis_state = genesis_state.unwrap_or_else(|| GenesisState {
        accounts: default_genesis_accounts(),
        authority_sudo_address: Address::from([0; 20]),
        ibc_sudo_address: Address::from([0; 20]),
        ibc_relayer_addresses: vec![],
        native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
        ibc_params: IBCParameters::default(),
        allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
        fees: default_fees(),
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

pub(crate) fn get_mock_tx(nonce: u32) -> SignedTransaction {
    let (alice_signing_key, _) = get_alice_signing_key_and_address();
    let tx = UnsignedTransaction {
        params: TransactionParams {
            nonce,
            chain_id: "test".to_string(),
        },
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes([0; 32]),
                data: vec![0x99],
                fee_asset_id: astria_core::primitive::v1::asset::default_native_asset_id(),
            }
            .into(),
        ],
    };

    tx.into_signed(&alice_signing_key)
}
