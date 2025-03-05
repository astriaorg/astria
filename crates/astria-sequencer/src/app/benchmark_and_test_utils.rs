use std::collections::HashMap;

use astria_core::{
    primitive::v1::asset::{
        Denom,
        IbcPrefixed,
    },
    protocol::{
        fees::v1::FeeComponents,
        genesis::v1::{
            Account,
            AddressPrefixes,
            GenesisAppState,
        },
        transaction::v1::action::{
            BridgeLock,
            BridgeSudoChange,
            BridgeTransfer,
            BridgeUnlock,
            FeeAssetChange,
            FeeChange,
            IbcRelayerChange,
            IbcSudoChange,
            Ics20Withdrawal,
            InitBridgeAccount,
            RecoverIbcClient,
            RollupDataSubmission,
            SudoAddressChange,
            Transfer,
            ValidatorUpdate,
        },
    },
    Protobuf,
};
use astria_eyre::eyre::WrapErr as _;
use cnidarium::{
    Snapshot,
    StateDelta,
    Storage,
};
use penumbra_ibc::IbcRelay;
use telemetry::Metrics as _;

use crate::{
    accounts::StateWriteExt as _,
    app::App,
    assets::StateWriteExt as _,
    benchmark_and_test_utils::{
        astria_address_from_hex_string,
        nria,
    },
    fees::StateWriteExt as _,
    mempool::Mempool,
    metrics::Metrics,
};

pub(crate) const ALICE_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
pub(crate) const BOB_ADDRESS: &str = "2269aca7b7c03c7d07345f83db4fababd1a05570";
pub(crate) const CAROL_ADDRESS: &str = "4e8846b82a8f31fd59265a9005959c4a030fc44c";
pub(crate) const JUDY_ADDRESS: &str = "989a77160cb0e96e2d168083ab72ffe89b41c199";
pub(crate) const TED_ADDRESS: &str = "4c4f91d8a918357ab5f6f19c1e179968fc39bb44";

pub(crate) fn address_prefixes() -> AddressPrefixes {
    AddressPrefixes::try_from_raw(
        astria_core::generated::astria::protocol::genesis::v1::AddressPrefixes {
            base: crate::benchmark_and_test_utils::ASTRIA_PREFIX.into(),
            ibc_compat: crate::benchmark_and_test_utils::ASTRIA_COMPAT_PREFIX.into(),
        },
    )
    .unwrap()
}

pub(crate) fn default_fees() -> astria_core::protocol::genesis::v1::GenesisFees {
    astria_core::protocol::genesis::v1::GenesisFees {
        transfer: Some(FeeComponents::<Transfer>::new(12, 0)),
        rollup_data_submission: Some(FeeComponents::<RollupDataSubmission>::new(32, 1)),
        init_bridge_account: Some(FeeComponents::<InitBridgeAccount>::new(48, 0)),
        // should reflect transfer fee
        bridge_lock: Some(FeeComponents::<BridgeLock>::new(12, 1)),
        bridge_sudo_change: Some(FeeComponents::<BridgeSudoChange>::new(24, 0)),
        ics20_withdrawal: Some(FeeComponents::<Ics20Withdrawal>::new(24, 0)),
        bridge_transfer: Some(FeeComponents::<BridgeTransfer>::new(24, 0)),
        // should reflect transfer fee
        bridge_unlock: Some(FeeComponents::<BridgeUnlock>::new(12, 0)),
        ibc_relay: Some(FeeComponents::<IbcRelay>::new(0, 0)),
        validator_update: Some(FeeComponents::<ValidatorUpdate>::new(0, 0)),
        fee_asset_change: Some(FeeComponents::<FeeAssetChange>::new(0, 0)),
        fee_change: FeeComponents::<FeeChange>::new(0, 0),
        ibc_relayer_change: Some(FeeComponents::<IbcRelayerChange>::new(0, 0)),
        sudo_address_change: Some(FeeComponents::<SudoAddressChange>::new(0, 0)),
        ibc_sudo_change: Some(FeeComponents::<IbcSudoChange>::new(0, 0)),
        recover_ibc_client: Some(FeeComponents::<RecoverIbcClient>::new(0, 0)),
    }
}

pub(crate) fn proto_genesis_state(
) -> astria_core::generated::astria::protocol::genesis::v1::GenesisAppState {
    use astria_core::generated::astria::protocol::genesis::v1::{
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
        native_asset_base_denomination: nria().to_string(),
        ibc_parameters: Some(IbcParameters {
            ibc_enabled: true,
            inbound_ics20_transfers_enabled: true,
            outbound_ics20_transfers_enabled: true,
        }),
        allowed_fee_assets: vec![nria().to_string()],
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
    let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
    let mempool = Mempool::new(metrics, 100);
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

pub(crate) fn mock_tx_cost(
    denom_0_cost: u128,
    denom_1_cost: u128,
    denom_2_cost: u128,
) -> HashMap<IbcPrefixed, u128> {
    let mut costs: HashMap<IbcPrefixed, u128> = HashMap::<IbcPrefixed, u128>::new();
    costs.insert(denom_0().to_ibc_prefixed(), denom_0_cost);
    costs.insert(denom_1().to_ibc_prefixed(), denom_1_cost);
    costs.insert(denom_2().to_ibc_prefixed(), denom_2_cost); // not present in balances

    // we don't sanitize the cost inputs
    costs.insert(denom_5().to_ibc_prefixed(), 0); // zero in balances also
    costs.insert(denom_6().to_ibc_prefixed(), 0); // not present in balances

    costs
}

pub(crate) const MOCK_SEQUENCE_FEE: u128 = 10;
pub(crate) fn denom_0() -> Denom {
    nria().into()
}

pub(crate) fn denom_1() -> Denom {
    "denom_1".parse().unwrap()
}

pub(crate) fn denom_2() -> Denom {
    "denom_2".parse().unwrap()
}

pub(crate) fn denom_3() -> Denom {
    "denom_3".parse().unwrap()
}

pub(crate) fn denom_4() -> Denom {
    "denom_4".parse().unwrap()
}

pub(crate) fn denom_5() -> Denom {
    "denom_5".parse().unwrap()
}

pub(crate) fn denom_6() -> Denom {
    "denom_6".parse().unwrap()
}

pub(crate) fn mock_balances(
    denom_0_balance: u128,
    denom_1_balance: u128,
) -> HashMap<IbcPrefixed, u128> {
    let mut balances = HashMap::<IbcPrefixed, u128>::new();
    if denom_0_balance != 0 {
        balances.insert(denom_0().to_ibc_prefixed(), denom_0_balance);
    }
    if denom_1_balance != 0 {
        balances.insert(denom_1().to_ibc_prefixed(), denom_1_balance);
    }
    // we don't sanitize the balance inputs
    balances.insert(denom_3().to_ibc_prefixed(), 100); // balance transaction costs won't have entry for
    balances.insert(denom_4().to_ibc_prefixed(), 0); // zero balance not in transaction
    balances.insert(denom_5().to_ibc_prefixed(), 0); // zero balance with corresponding zero cost

    balances
}

pub(crate) fn mock_state_put_account_balances(
    state: &mut StateDelta<Snapshot>,
    address: &[u8; 20],
    account_balances: HashMap<IbcPrefixed, u128>,
) {
    for (denom, balance) in account_balances {
        state.put_account_balance(address, &denom, balance).unwrap();
    }
}

pub(crate) fn mock_state_put_account_nonce(
    state: &mut StateDelta<Snapshot>,
    address: &[u8; 20],
    nonce: u32,
) {
    state.put_account_nonce(address, nonce).unwrap();
}

#[expect(clippy::too_many_lines, reason = "this is a test helper function")]
pub(crate) async fn mock_state_getter() -> StateDelta<Snapshot> {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();
    let mut state: StateDelta<cnidarium::Snapshot> = StateDelta::new(snapshot);

    // setup denoms
    state
        .put_ibc_asset(denom_0().unwrap_trace_prefixed())
        .unwrap();
    state
        .put_ibc_asset(denom_1().unwrap_trace_prefixed())
        .unwrap();
    state
        .put_ibc_asset(denom_2().unwrap_trace_prefixed())
        .unwrap();
    state
        .put_ibc_asset(denom_3().unwrap_trace_prefixed())
        .unwrap();
    state
        .put_ibc_asset(denom_4().unwrap_trace_prefixed())
        .unwrap();
    state
        .put_ibc_asset(denom_5().unwrap_trace_prefixed())
        .unwrap();
    state
        .put_ibc_asset(denom_6().unwrap_trace_prefixed())
        .unwrap();

    // setup tx fees
    let transfer_fees = FeeComponents::<Transfer>::new(0, 0);
    state
        .put_fees(transfer_fees)
        .wrap_err("failed to initiate transfer fee components")
        .unwrap();

    let rollup_data_submission_fees =
        FeeComponents::<RollupDataSubmission>::new(MOCK_SEQUENCE_FEE, 0);
    state
        .put_fees(rollup_data_submission_fees)
        .wrap_err("failed to initiate rollup data submission fee components")
        .unwrap();

    let ics20_withdrawal_fees = FeeComponents::<Ics20Withdrawal>::new(0, 0);
    state
        .put_fees(ics20_withdrawal_fees)
        .wrap_err("failed to initiate ics20 withdrawal fee components")
        .unwrap();

    let init_bridge_account_fees = FeeComponents::<InitBridgeAccount>::new(0, 0);
    state
        .put_fees(init_bridge_account_fees)
        .wrap_err("failed to initiate init bridge account fee components")
        .unwrap();

    let bridge_lock_fees = FeeComponents::<BridgeLock>::new(0, 0);
    state
        .put_fees(bridge_lock_fees)
        .wrap_err("failed to initiate bridge lock fee components")
        .unwrap();

    let bridge_unlock_fees = FeeComponents::<BridgeUnlock>::new(0, 0);
    state
        .put_fees(bridge_unlock_fees)
        .wrap_err("failed to initiate bridge unlock fee components")
        .unwrap();

    let bridge_sudo_change_fees = FeeComponents::<BridgeSudoChange>::new(0, 0);
    state
        .put_fees(bridge_sudo_change_fees)
        .wrap_err("failed to initiate bridge sudo change fee components")
        .unwrap();

    let ibc_relay_fees = FeeComponents::<IbcRelay>::new(0, 0);
    state
        .put_fees(ibc_relay_fees)
        .wrap_err("failed to initiate ibc relay fee components")
        .unwrap();

    let validator_update_fees = FeeComponents::<ValidatorUpdate>::new(0, 0);
    state
        .put_fees(validator_update_fees)
        .wrap_err("failed to initiate validator update fee components")
        .unwrap();

    let fee_asset_change_fees = FeeComponents::<FeeAssetChange>::new(0, 0);
    state
        .put_fees(fee_asset_change_fees)
        .wrap_err("failed to initiate fee asset change fee components")
        .unwrap();

    let fee_change_fees = FeeComponents::<FeeChange>::new(0, 0);
    state
        .put_fees(fee_change_fees)
        .wrap_err("failed to initiate fee change fees fee components")
        .unwrap();

    let ibc_relayer_change_fees = FeeComponents::<IbcRelayerChange>::new(0, 0);
    state
        .put_fees(ibc_relayer_change_fees)
        .wrap_err("failed to initiate ibc relayer change fee components")
        .unwrap();

    let sudo_address_change_fees = FeeComponents::<SudoAddressChange>::new(0, 0);
    state
        .put_fees(sudo_address_change_fees)
        .wrap_err("failed to initiate sudo address change fee components")
        .unwrap();

    let ibc_sudo_change_fees = FeeComponents::<IbcSudoChange>::new(0, 0);
    state
        .put_fees(ibc_sudo_change_fees)
        .wrap_err("failed to initiate ibc sudo change fee components")
        .unwrap();

    let recover_ibc_client_fees = FeeComponents::<RecoverIbcClient>::new(0, 0);
    state
        .put_fees(recover_ibc_client_fees)
        .wrap_err("failed to initiate recover ibc client fee components")
        .unwrap();

    // put denoms as allowed fee asset
    state.put_allowed_fee_asset(&denom_0()).unwrap();

    state
}
