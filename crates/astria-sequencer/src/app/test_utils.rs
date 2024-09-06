use std::{
    collections::HashMap,
    sync::Arc,
};

use astria_core::{
    crypto::SigningKey,
    primitive::v1::{
        asset::IbcPrefixed,
        RollupId,
    },
    protocol::{
        genesis::v1alpha1::{
            Account,
            AddressPrefixes,
            GenesisAppState,
        },
        transaction::v1alpha1::{
            action::{
                SequenceAction,
                ValidatorUpdate,
            },
            SignedTransaction,
            TransactionParams,
            UnsignedTransaction,
        },
    },
    Protobuf,
};
use bytes::Bytes;
use cnidarium::Storage;
use telemetry::Metrics as _;

use crate::{
    app::App,
    mempool::Mempool,
    metrics::Metrics,
    test_utils::astria_address_from_hex_string,
};

pub(crate) const ALICE_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
pub(crate) const BOB_ADDRESS: &str = "2269aca7b7c03c7d07345f83db4fababd1a05570";
pub(crate) const CAROL_ADDRESS: &str = "4e8846b82a8f31fd59265a9005959c4a030fc44c";
pub(crate) const JUDY_ADDRESS: &str = "989a77160cb0e96e2d168083ab72ffe89b41c199";
pub(crate) const TED_ADDRESS: &str = "4c4f91d8a918357ab5f6f19c1e179968fc39bb44";

#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) fn get_alice_signing_key() -> SigningKey {
    // this secret key corresponds to ALICE_ADDRESS
    let alice_secret_bytes: [u8; 32] =
        hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
            .unwrap()
            .try_into()
            .unwrap();
    SigningKey::from(alice_secret_bytes)
}

#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) fn get_bridge_signing_key() -> SigningKey {
    let bridge_secret_bytes: [u8; 32] =
        hex::decode("db4982e01f3eba9e74ac35422fcd49aa2b47c3c535345c7e7da5220fe3a0ce79")
            .unwrap()
            .try_into()
            .unwrap();
    SigningKey::from(bridge_secret_bytes)
}

pub(crate) fn default_genesis_accounts() -> Vec<Account> {
    vec![
        Account {
            address: astria_address_from_hex_string(ALICE_ADDRESS),
            balance: 10u128.pow(19),
        },
        Account {
            address: astria_address_from_hex_string(BOB_ADDRESS),
            balance: 10u128.pow(19),
        },
        Account {
            address: astria_address_from_hex_string(CAROL_ADDRESS),
            balance: 10u128.pow(19),
        },
    ]
}

#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) fn default_fees() -> astria_core::protocol::genesis::v1alpha1::Fees {
    astria_core::protocol::genesis::v1alpha1::Fees {
        transfer_base_fee: 12,
        sequence_base_fee: 32,
        sequence_byte_cost_multiplier: 1,
        init_bridge_account_base_fee: 48,
        bridge_lock_byte_cost_multiplier: 1,
        bridge_sudo_change_fee: 24,
        ics20_withdrawal_base_fee: 24,
    }
}

pub(crate) fn address_prefixes() -> AddressPrefixes {
    AddressPrefixes::try_from_raw(
        astria_core::generated::protocol::genesis::v1alpha1::AddressPrefixes {
            base: crate::test_utils::ASTRIA_PREFIX.into(),
            ibc_compat: crate::test_utils::ASTRIA_COMPAT_PREFIX.into(),
        },
    )
    .unwrap()
}

pub(crate) fn proto_genesis_state()
-> astria_core::generated::protocol::genesis::v1alpha1::GenesisAppState {
    use astria_core::generated::protocol::genesis::v1alpha1::{
        GenesisAppState,
        IbcParameters,
    };
    GenesisAppState {
        address_prefixes: Some(address_prefixes().to_raw()),
        accounts: default_genesis_accounts()
            .into_iter()
            .map(Protobuf::into_raw)
            .collect(),
        authority_sudo_address: Some(astria_address_from_hex_string(JUDY_ADDRESS).to_raw()),
        chain_id: "test-1".to_string(),
        ibc_sudo_address: Some(astria_address_from_hex_string(TED_ADDRESS).to_raw()),
        ibc_relayer_addresses: vec![],
        native_asset_base_denomination: crate::test_utils::nria().to_string(),
        ibc_parameters: Some(IbcParameters {
            ibc_enabled: true,
            inbound_ics20_transfers_enabled: true,
            outbound_ics20_transfers_enabled: true,
        }),
        allowed_fee_assets: vec![crate::test_utils::nria().to_string()],
        fees: Some(default_fees().to_raw()),
    }
}

pub(crate) fn genesis_state() -> GenesisAppState {
    proto_genesis_state().try_into().unwrap()
}

pub(crate) async fn initialize_app_with_storage(
    genesis_state: Option<GenesisAppState>,
    genesis_validators: Vec<ValidatorUpdate>,
) -> (App, Storage) {
    let storage = cnidarium::TempStorage::new()
        .await
        .expect("failed to create temp storage backing chain state");
    let snapshot = storage.latest_snapshot();
    let mempool = Mempool::new();
    let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
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

#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) async fn initialize_app(
    genesis_state: Option<GenesisAppState>,
    genesis_validators: Vec<ValidatorUpdate>,
) -> App {
    let (app, _storage) = initialize_app_with_storage(genesis_state, genesis_validators).await;
    app
}

#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) fn mock_tx(
    nonce: u32,
    signer: &SigningKey,
    rollup_name: &str,
) -> Arc<SignedTransaction> {
    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(nonce)
            .chain_id("test")
            .build(),
        actions: vec![
            SequenceAction {
                rollup_id: RollupId::from_unhashed_bytes(rollup_name.as_bytes()),
                data: Bytes::from_static(&[0x99]),
                fee_asset: "astria".parse().unwrap(),
            }
            .into(),
        ],
    };

    Arc::new(tx.into_signed(signer))
}

pub(crate) const DENOM_0: [u8; 32] = [1u8; 32];
pub(crate) const DENOM_1: [u8; 32] = [2u8; 32];
const DENOM_2: [u8; 32] = [3u8; 32];
pub(crate) const DENOM_3: [u8; 32] = [4u8; 32];
const DENOM_4: [u8; 32] = [5u8; 32];
const DENOM_5: [u8; 32] = [6u8; 32];
const DENOM_6: [u8; 32] = [7u8; 32];

pub(crate) fn mock_balances(
    denom_0_balance: u128,
    denom_1_balance: u128,
) -> HashMap<IbcPrefixed, u128> {
    let mut balances = HashMap::<IbcPrefixed, u128>::new();
    if denom_0_balance != 0 {
        balances.insert(IbcPrefixed::new(DENOM_0), denom_0_balance);
    }
    if denom_1_balance != 0 {
        balances.insert(IbcPrefixed::new(DENOM_1), denom_1_balance);
    }
    // we don't sanitize the balance inputs
    balances.insert(IbcPrefixed::new(DENOM_3), 100); // balance transaction costs won't have entry for
    balances.insert(IbcPrefixed::new(DENOM_4), 0); // zero balance not in transaction
    balances.insert(IbcPrefixed::new(DENOM_5), 0); // zero balance with corresponding zero cost 

    balances
}

pub(crate) fn mock_tx_cost(
    denom_0_cost: u128,
    denom_1_cost: u128,
    denom_2_cost: u128,
) -> HashMap<IbcPrefixed, u128> {
    let mut costs = HashMap::<IbcPrefixed, u128>::new();
    costs.insert(IbcPrefixed::new(DENOM_0), denom_0_cost);
    costs.insert(IbcPrefixed::new(DENOM_1), denom_1_cost);
    costs.insert(IbcPrefixed::new(DENOM_2), denom_2_cost); // not present in balances

    // we don't santize the cost inputs
    costs.insert(IbcPrefixed::new(DENOM_5), 0); // zero in balances also 
    costs.insert(IbcPrefixed::new(DENOM_6), 0); // not present in balances 

    costs
}
