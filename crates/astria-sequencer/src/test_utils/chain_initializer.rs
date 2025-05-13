use astria_core::{
    crypto::VerificationKey,
    generated::astria::protocol::genesis::v1::{
        Account as RawAccount,
        AddressPrefixes as RawAddressPrefixes,
        GenesisAppState as RawGenesisAppState,
        IbcParameters as RawIbcParameters,
    },
    primitive::v1::Address,
    protocol::{
        fees::v1::FeeComponents,
        genesis::v1::{
            GenesisAppState,
            GenesisFees,
        },
        transaction::v1::action::ValidatorUpdate,
    },
    Protobuf as _,
};

use super::{
    Fixture,
    ALICE,
    ALICE_ADDRESS,
    BOB,
    BOB_ADDRESS,
    CAROL,
    CAROL_ADDRESS,
    IBC_SUDO_ADDRESS,
    SUDO_ADDRESS,
    TEN_QUINTILLION,
};
use crate::test_utils::{
    astria_address,
    nria,
    ASTRIA_COMPAT_PREFIX,
    ASTRIA_PREFIX,
};

/// A helper struct to allow configuring the values used to initialize a test chain for use in
/// unit tests.
///
/// An instance can be constructed via [`Fixture::chain_initializer`].
///
/// By default, the following values are used:
///   * genesis app state:
///     * `chain_id`: "test"
///     * `address_prefixes`: "astria" and "astriacompat"
///     * `accounts`:  [`ALICE`], [`BOB`] and [`CAROL`], each with 10^19 nria
///     * `authority_sudo_address`: [`SUDO`]
///     * `ibc_sudo_address`: [`IBC_SUDO`]
///     * `ibc_relayer_addresses`: [`IBC_SUDO`]
///     * `native_asset_base_denomination`: nria
///     * `ibc_parameters`: all options enabled
///     * `allowed_fee_assets`: nria
///     * `fees`: all `Some`, all with different values to each other (see [`dummy_genesis_fees`])
///   * genesis validators: [`ALICE`], [`BOB`] and [`CAROL`], each with power 10
pub(crate) struct ChainInitializer<'a> {
    fixture: &'a mut Fixture,
    raw_genesis_app_state: RawGenesisAppState,
    genesis_validators: Vec<ValidatorUpdate>,
}

impl<'a> ChainInitializer<'a> {
    pub(super) fn new(fixture: &'a mut Fixture) -> Self {
        let genesis_validators = vec![
            ValidatorUpdate {
                power: 10,
                verification_key: ALICE.verification_key(),
                name: "Alice".parse().unwrap(),
            },
            ValidatorUpdate {
                power: 10,
                verification_key: BOB.verification_key(),
                name: "Bob".parse().unwrap(),
            },
            ValidatorUpdate {
                power: 10,
                verification_key: CAROL.verification_key(),
                name: "Carol".parse().unwrap(),
            },
        ];
        Self {
            fixture,
            raw_genesis_app_state: dummy_genesis_state(),
            genesis_validators,
        }
    }

    pub(super) fn legacy(fixture: &'a mut Fixture) -> Self {
        // Previously this was `TED_ADDRESS` in `benchmark_and_test_utils`.
        let ted_address =
            astria_address(&hex::decode("4c4f91d8a918357ab5f6f19c1e179968fc39bb44").unwrap());
        let raw_genesis_app_state = RawGenesisAppState {
            ibc_sudo_address: Some(ted_address.into_raw()),
            ibc_relayer_addresses: vec![],
            fees: Some(legacy_fees().into_raw()),
            ..dummy_genesis_state()
        };
        Self {
            fixture,
            raw_genesis_app_state,
            genesis_validators: vec![],
        }
    }

    /// Sets all genesis fees to `None` (except `FeeChange` which is non-optional).
    pub(crate) fn with_no_fees(mut self) -> Self {
        self.raw_genesis_app_state.fees = Some(
            GenesisFees {
                rollup_data_submission: None,
                transfer: None,
                ics20_withdrawal: None,
                init_bridge_account: None,
                bridge_lock: None,
                bridge_unlock: None,
                bridge_transfer: None,
                bridge_sudo_change: None,
                ibc_relay: None,
                validator_update: None,
                fee_asset_change: None,
                fee_change: FeeComponents::new(0, 0),
                ibc_relayer_change: None,
                sudo_address_change: None,
                ibc_sudo_change: None,
                recover_ibc_client: None,
                currency_pairs_change: None,
                markets_change: None,
            }
            .to_raw(),
        );
        self
    }

    /// Sets the `accounts` of genesis app state to the given values.
    pub(crate) fn with_genesis_accounts<I: IntoIterator<Item = (Address, u128)>>(
        mut self,
        genesis_accounts: I,
    ) -> Self {
        self.raw_genesis_app_state.accounts = genesis_accounts
            .into_iter()
            .map(|(address, balance)| RawAccount {
                address: Some(address.into_raw()),
                balance: Some(balance.into()),
            })
            .collect();
        self
    }

    /// Sets the `authority_sudo_address` of genesis app state to the given value.
    pub(crate) fn with_authority_sudo_address(mut self, address: Address) -> Self {
        self.raw_genesis_app_state.authority_sudo_address = Some(address.into_raw());
        self
    }

    /// Sets the `ibc_sudo_address` of genesis app state to the given value.
    pub(crate) fn with_ibc_sudo_address(mut self, address: Address) -> Self {
        self.raw_genesis_app_state.ibc_sudo_address = Some(address.into_raw());
        self
    }

    /// Sets the genesis validators to the given values.
    ///
    /// Their applied names are "Validator 0", "Validator 1", and so on.
    pub(crate) fn with_genesis_validators<I: IntoIterator<Item = (VerificationKey, u32)>>(
        mut self,
        validators: I,
    ) -> Self {
        self.genesis_validators = validators
            .into_iter()
            .enumerate()
            .map(|(index, (verification_key, power))| ValidatorUpdate {
                power,
                verification_key,
                name: format!("Validator {index}").parse().unwrap(),
            })
            .collect();
        self
    }

    /// Initializes the chain by calling `App::init_chain` and committing the resulting state
    /// changes.
    pub(crate) async fn init(self) {
        let ChainInitializer {
            fixture,
            raw_genesis_app_state,
            genesis_validators,
        } = self;

        assert!(
            fixture.genesis_app_state.is_none(),
            "can only init chain once"
        );

        let genesis_app_state = GenesisAppState::try_from_raw(raw_genesis_app_state).unwrap();
        fixture
            .app
            .init_chain(
                fixture.storage.clone(),
                genesis_app_state.clone(),
                genesis_validators,
                genesis_app_state.chain_id().to_string(),
            )
            .await
            .unwrap();
        fixture.app.commit(fixture.storage.clone()).await.unwrap();

        fixture.genesis_app_state = Some(genesis_app_state);
    }
}

fn dummy_genesis_state() -> RawGenesisAppState {
    let address_prefixes = RawAddressPrefixes {
        base: ASTRIA_PREFIX.into(),
        ibc_compat: ASTRIA_COMPAT_PREFIX.into(),
    };
    let accounts = vec![
        RawAccount {
            address: Some(ALICE_ADDRESS.to_raw()),
            balance: Some(TEN_QUINTILLION.into()),
        },
        RawAccount {
            address: Some(BOB_ADDRESS.to_raw()),
            balance: Some(TEN_QUINTILLION.into()),
        },
        RawAccount {
            address: Some(CAROL_ADDRESS.to_raw()),
            balance: Some(TEN_QUINTILLION.into()),
        },
    ];
    let ibc_parameters = RawIbcParameters {
        ibc_enabled: true,
        inbound_ics20_transfers_enabled: true,
        outbound_ics20_transfers_enabled: true,
    };

    RawGenesisAppState {
        chain_id: "test".to_string(),
        address_prefixes: Some(address_prefixes),
        accounts,
        authority_sudo_address: Some(SUDO_ADDRESS.to_raw()),
        ibc_sudo_address: Some(IBC_SUDO_ADDRESS.to_raw()),
        ibc_relayer_addresses: vec![IBC_SUDO_ADDRESS.to_raw()],
        native_asset_base_denomination: nria().to_string(),
        ibc_parameters: Some(ibc_parameters),
        allowed_fee_assets: vec![nria().to_string()],
        fees: Some(dummy_genesis_fees().to_raw()),
    }
}

fn dummy_genesis_fees() -> GenesisFees {
    GenesisFees {
        rollup_data_submission: Some(FeeComponents::new(1, 1001)),
        transfer: Some(FeeComponents::new(2, 1002)),
        ics20_withdrawal: Some(FeeComponents::new(3, 1003)),
        init_bridge_account: Some(FeeComponents::new(4, 1004)),
        bridge_lock: Some(FeeComponents::new(5, 1005)),
        bridge_unlock: Some(FeeComponents::new(6, 1006)),
        bridge_transfer: Some(FeeComponents::new(7, 1007)),
        bridge_sudo_change: Some(FeeComponents::new(8, 1008)),
        ibc_relay: Some(FeeComponents::new(9, 1009)),
        validator_update: Some(FeeComponents::new(10, 1010)),
        fee_asset_change: Some(FeeComponents::new(11, 1011)),
        fee_change: FeeComponents::new(12, 1012),
        ibc_relayer_change: Some(FeeComponents::new(13, 1013)),
        sudo_address_change: Some(FeeComponents::new(14, 1014)),
        ibc_sudo_change: Some(FeeComponents::new(15, 1015)),
        recover_ibc_client: Some(FeeComponents::new(16, 1016)),
        currency_pairs_change: Some(FeeComponents::new(17, 1017)),
        markets_change: Some(FeeComponents::new(18, 1018)),
    }
}

fn legacy_fees() -> GenesisFees {
    GenesisFees {
        transfer: Some(FeeComponents::new(12, 0)),
        rollup_data_submission: Some(FeeComponents::new(32, 1)),
        init_bridge_account: Some(FeeComponents::new(48, 0)),
        bridge_lock: Some(FeeComponents::new(12, 1)),
        bridge_sudo_change: Some(FeeComponents::new(24, 0)),
        ics20_withdrawal: Some(FeeComponents::new(24, 0)),
        // NOTE: These were set to `Some(FeeComponents::new(24, 0))`, but
        // `FeesComponent::init_chain` didn't store them.
        bridge_transfer: None,
        bridge_unlock: Some(FeeComponents::new(12, 0)),
        ibc_relay: Some(FeeComponents::new(0, 0)),
        validator_update: Some(FeeComponents::new(0, 0)),
        fee_asset_change: Some(FeeComponents::new(0, 0)),
        fee_change: FeeComponents::new(0, 0),
        ibc_relayer_change: Some(FeeComponents::new(0, 0)),
        sudo_address_change: Some(FeeComponents::new(0, 0)),
        ibc_sudo_change: Some(FeeComponents::new(0, 0)),
        recover_ibc_client: Some(FeeComponents::new(0, 0)),
        currency_pairs_change: Some(FeeComponents::new(0, 0)),
        markets_change: Some(FeeComponents::new(0, 0)),
    }
}
