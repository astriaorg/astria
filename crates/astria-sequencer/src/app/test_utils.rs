use std::{
    collections::HashMap,
    sync::Arc,
};

use astria_core::{
    crypto::SigningKey,
    primitive::v1::{
        asset::{
            Denom,
            IbcPrefixed,
        },
        RollupId,
    },
    protocol::{
        fees::v1::{
            BridgeLockFeeComponents,
            BridgeSudoChangeFeeComponents,
            BridgeUnlockFeeComponents,
            FeeAssetChangeFeeComponents,
            FeeChangeFeeComponents,
            IbcRelayFeeComponents,
            IbcRelayerChangeFeeComponents,
            IbcSudoChangeFeeComponents,
            Ics20WithdrawalFeeComponents,
            InitBridgeAccountFeeComponents,
            RollupDataSubmissionFeeComponents,
            SudoAddressChangeFeeComponents,
            TransferFeeComponents,
            ValidatorUpdateFeeComponents,
        },
        genesis::v1::{
            Account,
            AddressPrefixes,
            GenesisAppState,
        },
        transaction::v1::{
            action::{
                group::Group,
                FeeAssetChange,
                InitBridgeAccount,
                RollupDataSubmission,
                SudoAddressChange,
                ValidatorUpdate,
            },
            Action,
            Transaction,
            TransactionBody,
        },
    },
    Protobuf,
};
use astria_eyre::eyre::WrapErr as _;
use bytes::Bytes;
use cnidarium::{
    Snapshot,
    StateDelta,
    Storage,
};
use indexmap::IndexMap;
use telemetry::Metrics as _;

use crate::{
    accounts::StateWriteExt,
    app::{
        vote_extension,
        App,
    },
    assets::StateWriteExt as AssetStateWriteExt,
    fees::StateWriteExt as _,
    mempool::Mempool,
    metrics::Metrics,
    test_utils::{
        astria_address_from_hex_string,
        nria,
    },
};

pub(crate) const ALICE_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
pub(crate) const BOB_ADDRESS: &str = "2269aca7b7c03c7d07345f83db4fababd1a05570";
pub(crate) const CAROL_ADDRESS: &str = "4e8846b82a8f31fd59265a9005959c4a030fc44c";
pub(crate) const JUDY_ADDRESS: &str = "989a77160cb0e96e2d168083ab72ffe89b41c199";
pub(crate) const TED_ADDRESS: &str = "4c4f91d8a918357ab5f6f19c1e179968fc39bb44";

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
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

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) fn get_bob_signing_key() -> SigningKey {
    // this secret key corresponds to ALICE_ADDRESS
    let bob_secret_bytes: [u8; 32] =
        hex::decode("b70fd3b99cab2d98dbd73602deb026b9cdc9bb7b85d35f0bbb81b17c78923dd0")
            .unwrap()
            .try_into()
            .unwrap();
    SigningKey::from(bob_secret_bytes)
}

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) fn get_carol_signing_key() -> SigningKey {
    // this secret key corresponds to ALICE_ADDRESS
    let carol_secret_bytes: [u8; 32] =
        hex::decode("0e951afdcbefc420fe6f71b82b0c28c11eb6ee5d95be0886ce9dbf6fa512debc")
            .unwrap()
            .try_into()
            .unwrap();
    SigningKey::from(carol_secret_bytes)
}

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) fn get_judy_signing_key() -> SigningKey {
    // this secret key corresponds to ALICE_ADDRESS
    let judy_secret_bytes: [u8; 32] =
        hex::decode("3b2a05a2168952a102dcc07f39b9e385a45b9c2a9b6e3d06acf46fb39fd14019")
            .unwrap()
            .try_into()
            .unwrap();
    SigningKey::from(judy_secret_bytes)
}

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
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

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) fn default_fees() -> astria_core::protocol::genesis::v1::GenesisFees {
    astria_core::protocol::genesis::v1::GenesisFees {
        transfer: Some(TransferFeeComponents {
            base: 12,
            multiplier: 0,
        }),
        rollup_data_submission: Some(RollupDataSubmissionFeeComponents {
            base: 32,
            multiplier: 1,
        }),
        init_bridge_account: Some(InitBridgeAccountFeeComponents {
            base: 48,
            multiplier: 0,
        }),
        bridge_lock: Some(BridgeLockFeeComponents {
            base: 12, // should reflect transfer fee
            multiplier: 1,
        }),
        bridge_sudo_change: Some(BridgeSudoChangeFeeComponents {
            base: 24,
            multiplier: 0,
        }),
        ics20_withdrawal: Some(Ics20WithdrawalFeeComponents {
            base: 24,
            multiplier: 0,
        }),
        bridge_unlock: Some(BridgeUnlockFeeComponents {
            base: 12, // should reflect transfer fee
            multiplier: 0,
        }),
        ibc_relay: Some(IbcRelayFeeComponents {
            base: 0,
            multiplier: 0,
        }),
        validator_update: Some(ValidatorUpdateFeeComponents {
            base: 0,
            multiplier: 0,
        }),
        fee_asset_change: Some(FeeAssetChangeFeeComponents {
            base: 0,
            multiplier: 0,
        }),
        fee_change: FeeChangeFeeComponents {
            base: 0,
            multiplier: 0,
        },
        ibc_relayer_change: Some(IbcRelayerChangeFeeComponents {
            base: 0,
            multiplier: 0,
        }),
        sudo_address_change: Some(SudoAddressChangeFeeComponents {
            base: 0,
            multiplier: 0,
        }),
        ibc_sudo_change: Some(IbcSudoChangeFeeComponents {
            base: 0,
            multiplier: 0,
        }),
    }
}

pub(crate) fn address_prefixes() -> AddressPrefixes {
    AddressPrefixes::try_from_raw(
        astria_core::generated::protocol::genesis::v1::AddressPrefixes {
            base: crate::test_utils::ASTRIA_PREFIX.into(),
            ibc_compat: crate::test_utils::ASTRIA_COMPAT_PREFIX.into(),
        },
    )
    .unwrap()
}

pub(crate) fn proto_genesis_state() -> astria_core::generated::protocol::genesis::v1::GenesisAppState
{
    use astria_core::{
        generated::protocol::genesis::v1::{
            GenesisAppState,
            IbcParameters,
            SlinkyGenesis,
        },
        slinky::market_map::v1::{
            MarketMap,
            Params,
        },
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
        slinky: Some(SlinkyGenesis {
            market_map: Some(
                astria_core::slinky::market_map::v1::GenesisState {
                    market_map: MarketMap {
                        markets: IndexMap::new(),
                    },
                    last_updated: 0,
                    params: Params {
                        market_authorities: vec![],
                        admin: astria_address_from_hex_string(ALICE_ADDRESS),
                    },
                }
                .into_raw(),
            ),
            oracle: Some(
                astria_core::generated::astria_vendored::slinky::oracle::v1::GenesisState {
                    currency_pair_genesis: vec![],
                    next_id: 0,
                },
            ),
        }),
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
    let mut app = App::new(
        snapshot,
        mempool,
        vote_extension::Handler::new(None),
        metrics,
    )
    .await
    .unwrap();

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

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) async fn initialize_app(
    genesis_state: Option<GenesisAppState>,
    genesis_validators: Vec<ValidatorUpdate>,
) -> App {
    let (app, _storage) = initialize_app_with_storage(genesis_state, genesis_validators).await;
    app
}

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) struct MockTxBuilder {
    nonce: u32,
    signer: SigningKey,
    chain_id: String,
    group: Group,
}

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
#[cfg_attr(feature = "benchmark", allow(dead_code))]
impl MockTxBuilder {
    pub(crate) fn new() -> Self {
        Self {
            chain_id: "test".to_string(),
            nonce: 0,
            signer: get_alice_signing_key(),
            group: Group::BundleableGeneral,
        }
    }

    pub(crate) fn nonce(self, nonce: u32) -> Self {
        Self {
            nonce,
            ..self
        }
    }

    pub(crate) fn signer(self, signer: SigningKey) -> Self {
        Self {
            signer,
            ..self
        }
    }

    pub(crate) fn chain_id(self, chain_id: &str) -> Self {
        Self {
            chain_id: chain_id.to_string(),
            ..self
        }
    }

    pub(crate) fn group(self, group: Group) -> Self {
        Self {
            group,
            ..self
        }
    }

    pub(crate) fn build(self) -> Arc<Transaction> {
        let action: Action = match self.group {
            Group::BundleableGeneral => RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes("rollup-id"),
                data: Bytes::from_static(&[0x99]),
                fee_asset: denom_0(),
            }
            .into(),
            Group::UnbundleableGeneral => InitBridgeAccount {
                rollup_id: RollupId::from_unhashed_bytes("rollup-id"),
                asset: denom_0(),
                fee_asset: denom_0(),
                sudo_address: None,
                withdrawer_address: None,
            }
            .into(),
            Group::BundleableSudo => FeeAssetChange::Addition(denom_0()).into(),
            Group::UnbundleableSudo => SudoAddressChange {
                new_address: astria_address_from_hex_string(JUDY_ADDRESS),
            }
            .into(),
        };

        assert!(
            action.group() == self.group,
            "action group mismatch: wanted {:?}, got {:?}",
            self.group,
            action.group()
        );

        let tx = TransactionBody::builder()
            .actions(vec![action])
            .chain_id(self.chain_id)
            .nonce(self.nonce)
            .try_build()
            .unwrap();

        Arc::new(tx.sign(&self.signer))
    }
}

pub(crate) const MOCK_SEQUENCE_FEE: u128 = 10;
pub(crate) fn denom_0() -> Denom {
    nria().into()
}

pub(crate) fn denom_1() -> Denom {
    "denom_1".parse().unwrap()
}

fn denom_2() -> Denom {
    "denom_2".parse().unwrap()
}

pub(crate) fn denom_3() -> Denom {
    "denom_3".parse().unwrap()
}

fn denom_4() -> Denom {
    "denom_4".parse().unwrap()
}

fn denom_5() -> Denom {
    "denom_5".parse().unwrap()
}

fn denom_6() -> Denom {
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

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) fn mock_state_put_account_balances(
    state: &mut StateDelta<Snapshot>,
    address: &[u8; 20],
    account_balances: HashMap<IbcPrefixed, u128>,
) {
    for (denom, balance) in account_balances {
        state.put_account_balance(address, &denom, balance).unwrap();
    }
}

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) fn mock_state_put_account_nonce(
    state: &mut StateDelta<Snapshot>,
    address: &[u8; 20],
    nonce: u32,
) {
    state.put_account_nonce(address, nonce).unwrap();
}

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::too_many_lines,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
#[cfg_attr(feature = "benchmark", allow(dead_code))]
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
    let transfer_fees = TransferFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_transfer_fees(transfer_fees)
        .wrap_err("failed to initiate transfer fee components")
        .unwrap();

    let rollup_data_submission_fees = RollupDataSubmissionFeeComponents {
        base: MOCK_SEQUENCE_FEE,
        multiplier: 0,
    };
    state
        .put_rollup_data_submission_fees(rollup_data_submission_fees)
        .wrap_err("failed to initiate sequence action fee components")
        .unwrap();

    let ics20_withdrawal_fees = Ics20WithdrawalFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_ics20_withdrawal_fees(ics20_withdrawal_fees)
        .wrap_err("failed to initiate ics20 withdrawal fee components")
        .unwrap();

    let init_bridge_account_fees = InitBridgeAccountFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_init_bridge_account_fees(init_bridge_account_fees)
        .wrap_err("failed to initiate init bridge account fee components")
        .unwrap();

    let bridge_lock_fees = BridgeLockFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_bridge_lock_fees(bridge_lock_fees)
        .wrap_err("failed to initiate bridge lock fee components")
        .unwrap();

    let bridge_unlock_fees = BridgeUnlockFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_bridge_unlock_fees(bridge_unlock_fees)
        .wrap_err("failed to initiate bridge unlock fee components")
        .unwrap();

    let bridge_sudo_change_fees = BridgeSudoChangeFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_bridge_sudo_change_fees(bridge_sudo_change_fees)
        .wrap_err("failed to initiate bridge sudo change fee components")
        .unwrap();

    let ibc_relay_fees = IbcRelayFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_ibc_relay_fees(ibc_relay_fees)
        .wrap_err("failed to initiate ibc relay fee components")
        .unwrap();

    let validator_update_fees = ValidatorUpdateFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_validator_update_fees(validator_update_fees)
        .wrap_err("failed to initiate validator update fee components")
        .unwrap();

    let fee_asset_change_fees = FeeAssetChangeFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_fee_asset_change_fees(fee_asset_change_fees)
        .wrap_err("failed to initiate fee asset change fee components")
        .unwrap();

    let fee_change_fees = FeeChangeFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_fee_change_fees(fee_change_fees)
        .wrap_err("failed to initiate fee change fees fee components")
        .unwrap();

    let ibc_relayer_change_fees = IbcRelayerChangeFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_ibc_relayer_change_fees(ibc_relayer_change_fees)
        .wrap_err("failed to initiate ibc relayer change fee components")
        .unwrap();

    let sudo_address_change_fees = SudoAddressChangeFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_sudo_address_change_fees(sudo_address_change_fees)
        .wrap_err("failed to initiate sudo address change fee components")
        .unwrap();

    let ibc_sudo_change_fees = IbcSudoChangeFeeComponents {
        base: 0,
        multiplier: 0,
    };
    state
        .put_ibc_sudo_change_fees(ibc_sudo_change_fees)
        .wrap_err("failed to initiate ibc sudo change fee components")
        .unwrap();

    // put denoms as allowed fee asset
    state.put_allowed_fee_asset(&denom_0()).unwrap();

    state
}
