use std::convert::Infallible;

use borsh::BorshSerialize;
pub use penumbra_ibc::params::IBCParameters;
use penumbra_ibc::IbcRelay;

use crate::{
    generated::astria::protocol::genesis::v1 as raw,
    oracles::price_feed::{
        market_map,
        oracle,
    },
    primitive::v1::{
        asset::{
            self,
            denom::ParseTracePrefixedError,
            ParseDenomError,
        },
        Address,
        AddressError,
        Bech32,
        Bech32m,
    },
    protocol::{
        fees::v1::{
            FeeComponentError,
            FeeComponents,
        },
        transaction::v1::action::{
            AddCurrencyPairs,
            BridgeLock,
            BridgeSudoChange,
            BridgeUnlock,
            FeeAssetChange,
            FeeChange,
            IbcRelayerChange,
            IbcSudoChange,
            Ics20Withdrawal,
            InitBridgeAccount,
            RemoveCurrencyPairs,
            RollupDataSubmission,
            SudoAddressChange,
            Transfer,
            ValidatorUpdate,
        },
    },
    Protobuf,
};

#[derive(Clone, Debug, BorshSerialize)]
pub struct PriceFeedGenesis {
    market_map: market_map::v2::GenesisState,
    oracle: oracle::v2::GenesisState,
}

impl PriceFeedGenesis {
    #[must_use]
    pub fn market_map(&self) -> &market_map::v2::GenesisState {
        &self.market_map
    }

    #[must_use]
    pub fn oracle(&self) -> &oracle::v2::GenesisState {
        &self.oracle
    }
}

impl Protobuf for PriceFeedGenesis {
    type Error = PriceFeedGenesisError;
    type Raw = raw::PriceFeedGenesis;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            market_map,
            oracle,
        } = raw;
        let market_map = market_map
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("market_map"))
            .and_then(|market_map| {
                market_map::v2::GenesisState::try_from_raw_ref(market_map)
                    .map_err(Self::Error::market_map)
            })?;
        let oracle = oracle
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("oracle"))
            .and_then(|oracle| {
                oracle::v2::GenesisState::try_from_raw_ref(oracle).map_err(Self::Error::oracle)
            })?;
        Ok(Self {
            market_map,
            oracle,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            market_map,
            oracle,
        } = self;
        Self::Raw {
            market_map: Some(market_map.to_raw()),
            oracle: Some(oracle.to_raw()),
        }
    }
}

impl TryFrom<raw::PriceFeedGenesis> for PriceFeedGenesis {
    type Error = <Self as Protobuf>::Error;

    fn try_from(value: raw::PriceFeedGenesis) -> Result<Self, Self::Error> {
        Self::try_from_raw(value)
    }
}

impl From<PriceFeedGenesis> for raw::PriceFeedGenesis {
    fn from(value: PriceFeedGenesis) -> Self {
        value.into_raw()
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct PriceFeedGenesisError(PriceFeedGenesisErrorKind);

impl PriceFeedGenesisError {
    fn field_not_set(name: &'static str) -> Self {
        Self(PriceFeedGenesisErrorKind::FieldNotSet {
            name,
        })
    }

    fn market_map(source: market_map::v2::GenesisStateError) -> Self {
        Self(PriceFeedGenesisErrorKind::MarketMap {
            source,
        })
    }

    fn oracle(source: oracle::v2::GenesisStateError) -> Self {
        Self(PriceFeedGenesisErrorKind::Oracle {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed ensuring invariants of {}", PriceFeedGenesis::full_name())]
enum PriceFeedGenesisErrorKind {
    #[error("field was not set: `{name}`")]
    FieldNotSet { name: &'static str },
    #[error("`market_map` field was invalid")]
    MarketMap {
        source: market_map::v2::GenesisStateError,
    },
    #[error("`oracle` field was invalid")]
    Oracle {
        source: oracle::v2::GenesisStateError,
    },
}

/// The genesis state of Astria's Sequencer.
///
/// Verified to only contain valid fields (right now, addresses that have the same base prefix
/// as set in `GenesisState::address_prefixes::base`).
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(try_from = "raw::GenesisAppState", into = "raw::GenesisAppState")
)]
pub struct GenesisAppState {
    chain_id: String,
    address_prefixes: AddressPrefixes,
    accounts: Vec<Account>,
    authority_sudo_address: crate::primitive::v1::Address,
    ibc_sudo_address: crate::primitive::v1::Address,
    ibc_relayer_addresses: Vec<crate::primitive::v1::Address>,
    native_asset_base_denomination: Option<asset::TracePrefixed>,
    ibc_parameters: IBCParameters,
    allowed_fee_assets: Vec<asset::Denom>,
    fees: GenesisFees,
    price_feed: Option<PriceFeedGenesis>,
}

impl GenesisAppState {
    #[must_use]
    pub fn address_prefixes(&self) -> &AddressPrefixes {
        &self.address_prefixes
    }

    #[must_use]
    pub fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    #[must_use]
    pub fn authority_sudo_address(&self) -> &Address {
        &self.authority_sudo_address
    }

    #[must_use]
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    #[must_use]
    pub fn ibc_sudo_address(&self) -> &Address {
        &self.ibc_sudo_address
    }

    #[must_use]
    pub fn ibc_relayer_addresses(&self) -> &[Address] {
        &self.ibc_relayer_addresses
    }

    #[must_use]
    pub fn native_asset_base_denomination(&self) -> Option<&asset::TracePrefixed> {
        self.native_asset_base_denomination.as_ref()
    }

    #[must_use]
    pub fn ibc_parameters(&self) -> &IBCParameters {
        &self.ibc_parameters
    }

    #[must_use]
    pub fn allowed_fee_assets(&self) -> &[asset::Denom] {
        &self.allowed_fee_assets
    }

    #[must_use]
    pub fn fees(&self) -> &GenesisFees {
        &self.fees
    }

    #[must_use]
    pub fn price_feed(&self) -> &Option<PriceFeedGenesis> {
        &self.price_feed
    }

    fn ensure_address_has_base_prefix(
        &self,
        address: &Address,
        field: &str,
    ) -> Result<(), Box<AddressDoesNotMatchBase>> {
        if self.address_prefixes.base != address.prefix() {
            return Err(Box::new(AddressDoesNotMatchBase {
                base_prefix: self.address_prefixes.base.clone(),
                address: *address,
                field: field.to_string(),
            }));
        }
        Ok(())
    }

    fn ensure_all_addresses_have_base_prefix(&self) -> Result<(), Box<AddressDoesNotMatchBase>> {
        for (i, account) in self.accounts.iter().enumerate() {
            self.ensure_address_has_base_prefix(
                &account.address,
                &format!(".accounts[{i}].address"),
            )?;
        }
        self.ensure_address_has_base_prefix(
            &self.authority_sudo_address,
            ".authority_sudo_address",
        )?;
        self.ensure_address_has_base_prefix(&self.ibc_sudo_address, ".ibc_sudo_address")?;
        for (i, address) in self.ibc_relayer_addresses.iter().enumerate() {
            self.ensure_address_has_base_prefix(address, &format!(".ibc_relayer_addresses[{i}]"))?;
        }

        if let Some(price_feed) = &self.price_feed {
            for (i, address) in price_feed
                .market_map
                .params
                .market_authorities
                .iter()
                .enumerate()
            {
                self.ensure_address_has_base_prefix(
                    address,
                    &format!(".market_map.params.market_authorities[{i}]"),
                )?;
            }
            self.ensure_address_has_base_prefix(
                &price_feed.market_map.params.admin,
                ".market_map.params.admin",
            )?;
        }

        Ok(())
    }
}

impl Protobuf for GenesisAppState {
    type Error = GenesisAppStateError;
    type Raw = raw::GenesisAppState;

    // TODO (https://github.com/astriaorg/astria/issues/1580): remove this once Rust is upgraded to/past 1.83
    #[expect(
        clippy::allow_attributes,
        clippy::allow_attributes_without_reason,
        reason = "false positive on `allowed_fee_assets` due to \"allow\" in the name"
    )]
    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            address_prefixes,
            accounts,
            authority_sudo_address,
            chain_id,
            ibc_sudo_address,
            ibc_relayer_addresses,
            native_asset_base_denomination,
            ibc_parameters,
            allowed_fee_assets,
            fees,
            price_feed,
        } = raw;
        let address_prefixes = address_prefixes
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("address_prefixes"))
            .and_then(|aps| {
                AddressPrefixes::try_from_raw_ref(aps).map_err(Self::Error::address_prefixes)
            })?;
        let accounts = accounts
            .iter()
            .map(Account::try_from_raw_ref)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Self::Error::accounts)?;

        let authority_sudo_address = authority_sudo_address
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("authority_sudo_address"))
            .and_then(|addr| {
                Address::try_from_raw_ref(addr).map_err(Self::Error::authority_sudo_address)
            })?;
        let ibc_sudo_address = ibc_sudo_address
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("ibc_sudo_address"))
            .and_then(|addr| {
                Address::try_from_raw_ref(addr).map_err(Self::Error::ibc_sudo_address)
            })?;

        let ibc_relayer_addresses = ibc_relayer_addresses
            .iter()
            .map(Address::try_from_raw_ref)
            .collect::<Result<_, _>>()
            .map_err(Self::Error::ibc_relayer_addresses)?;

        let native_asset_base_denomination = if native_asset_base_denomination.is_empty() {
            None
        } else {
            Some(
                native_asset_base_denomination
                    .parse()
                    .map_err(Self::Error::native_asset_base_denomination),
            )
            .transpose()?
        };

        let ibc_parameters = {
            let params = ibc_parameters
                .as_ref()
                .ok_or_else(|| Self::Error::field_not_set("ibc_parameters"))?;
            IBCParameters::try_from_raw_ref(params).expect("conversion is infallible")
        };

        let allowed_fee_assets = allowed_fee_assets
            .iter()
            .map(|asset| asset.parse())
            .collect::<Result<_, _>>()
            .map_err(Self::Error::allowed_fee_assets)?;

        let fees = fees
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("fees"))
            .and_then(|fees| GenesisFees::try_from_raw_ref(fees).map_err(Self::Error::fees))?;

        let price_feed = if let Some(price_feed) = price_feed {
            Some(PriceFeedGenesis::try_from_raw_ref(price_feed).map_err(Self::Error::price_feed)?)
        } else {
            None
        };

        let this = Self {
            address_prefixes,
            accounts,
            authority_sudo_address,
            chain_id: chain_id.clone(),
            ibc_sudo_address,
            ibc_relayer_addresses,
            native_asset_base_denomination,
            ibc_parameters,
            allowed_fee_assets,
            fees,
            price_feed,
        };
        this.ensure_all_addresses_have_base_prefix()
            .map_err(Self::Error::address_does_not_match_base)?;
        Ok(this)
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            address_prefixes,
            accounts,
            authority_sudo_address,
            chain_id,
            ibc_sudo_address,
            ibc_relayer_addresses,
            native_asset_base_denomination,
            ibc_parameters,
            allowed_fee_assets,
            fees,
            price_feed,
        } = self;
        Self::Raw {
            address_prefixes: Some(address_prefixes.to_raw()),
            accounts: accounts.iter().map(Account::to_raw).collect(),
            authority_sudo_address: Some(authority_sudo_address.to_raw()),
            chain_id: chain_id.clone(),
            ibc_sudo_address: Some(ibc_sudo_address.to_raw()),
            ibc_relayer_addresses: ibc_relayer_addresses.iter().map(Address::to_raw).collect(),
            native_asset_base_denomination: native_asset_base_denomination
                .as_ref()
                .map_or(String::new(), ToString::to_string),
            ibc_parameters: Some(ibc_parameters.to_raw()),
            allowed_fee_assets: allowed_fee_assets.iter().map(ToString::to_string).collect(),
            fees: Some(fees.to_raw()),
            price_feed: price_feed.as_ref().map(PriceFeedGenesis::to_raw),
        }
    }
}

impl TryFrom<raw::GenesisAppState> for GenesisAppState {
    type Error = <Self as Protobuf>::Error;

    fn try_from(value: raw::GenesisAppState) -> Result<Self, Self::Error> {
        Self::try_from_raw(value)
    }
}

impl From<GenesisAppState> for raw::GenesisAppState {
    fn from(value: GenesisAppState) -> Self {
        value.into_raw()
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct GenesisAppStateError(GenesisAppStateErrorKind);

impl GenesisAppStateError {
    fn accounts(source: AccountError) -> Self {
        Self(GenesisAppStateErrorKind::Accounts {
            source,
        })
    }

    fn address_prefixes(source: AddressPrefixesError) -> Self {
        Self(GenesisAppStateErrorKind::AddressPrefixes {
            source,
        })
    }

    fn address_does_not_match_base(source: Box<AddressDoesNotMatchBase>) -> Self {
        Self(GenesisAppStateErrorKind::AddressDoesNotMatchBase {
            source,
        })
    }

    fn allowed_fee_assets(source: ParseDenomError) -> Self {
        Self(GenesisAppStateErrorKind::AllowedFeeAssets {
            source,
        })
    }

    fn authority_sudo_address(source: AddressError) -> Self {
        Self(GenesisAppStateErrorKind::AuthoritySudoAddress {
            source,
        })
    }

    fn fees(source: FeesError) -> Self {
        Self(GenesisAppStateErrorKind::Fees {
            source,
        })
    }

    fn field_not_set(name: &'static str) -> Self {
        Self(GenesisAppStateErrorKind::FieldNotSet {
            name,
        })
    }

    fn ibc_relayer_addresses(source: AddressError) -> Self {
        Self(GenesisAppStateErrorKind::IbcRelayerAddresses {
            source,
        })
    }

    fn ibc_sudo_address(source: AddressError) -> Self {
        Self(GenesisAppStateErrorKind::IbcSudoAddress {
            source,
        })
    }

    fn native_asset_base_denomination(source: ParseTracePrefixedError) -> Self {
        Self(GenesisAppStateErrorKind::NativeAssetBaseDenomination {
            source,
        })
    }

    fn price_feed(source: PriceFeedGenesisError) -> Self {
        Self(GenesisAppStateErrorKind::PriceFeed {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed ensuring invariants of {}", GenesisAppState::full_name())]
enum GenesisAppStateErrorKind {
    #[error("`accounts` field was invalid")]
    Accounts { source: AccountError },
    #[error("`address_prefixes` field was invalid")]
    AddressPrefixes { source: AddressPrefixesError },
    #[error("one of the provided addresses did not match the provided base prefix")]
    AddressDoesNotMatchBase {
        source: Box<AddressDoesNotMatchBase>,
    },
    #[error("`allowed_fee_assets` field was invalid")]
    AllowedFeeAssets { source: ParseDenomError },
    #[error("`authority_sudo_address` field was invalid")]
    AuthoritySudoAddress { source: AddressError },
    #[error("`fees` field was invalid")]
    Fees { source: FeesError },
    #[error("`ibc_sudo_address` field was invalid")]
    IbcSudoAddress { source: AddressError },
    #[error("`ibc_relayer_addresses` field was invalid")]
    IbcRelayerAddresses { source: AddressError },
    #[error("field was not set: `{name}`")]
    FieldNotSet { name: &'static str },
    #[error("`native_asset_base_denomination` field was invalid")]
    NativeAssetBaseDenomination { source: ParseTracePrefixedError },
    #[error("`price_feed` field was invalid")]
    PriceFeed { source: PriceFeedGenesisError },
}

#[derive(Debug, thiserror::Error)]
#[error("address `{address}` at `{field}` does not have `{base_prefix}`")]
struct AddressDoesNotMatchBase {
    base_prefix: String,
    address: Address,
    field: String,
}

#[derive(Clone, Copy, Debug)]
pub struct Account {
    pub address: Address,
    pub balance: u128,
}

impl Protobuf for Account {
    type Error = AccountError;
    type Raw = raw::Account;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            address,
            balance,
        } = raw;
        let address = address
            .as_ref()
            .ok_or_else(|| AccountError::field_not_set("address"))
            .and_then(|addr| Address::try_from_raw_ref(addr).map_err(Self::Error::address))?;
        let balance = balance
            .ok_or_else(|| AccountError::field_not_set("balance"))
            .map(Into::into)?;
        Ok(Self {
            address,
            balance,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            address,
            balance,
        } = self;
        Self::Raw {
            address: Some(address.to_raw()),
            balance: Some((*balance).into()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct AccountError(AccountErrorKind);

impl AccountError {
    fn address(source: AddressError) -> Self {
        Self(AccountErrorKind::Address {
            source,
        })
    }

    fn field_not_set(name: &'static str) -> Self {
        Self(AccountErrorKind::FieldNotSet {
            name,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed ensuring invariants of {}", Account::full_name())]
enum AccountErrorKind {
    #[error("`address` field was invalid")]
    Address { source: AddressError },
    #[error("field was not set: `{name}`")]
    FieldNotSet { name: &'static str },
}

/// The address prefixes used by the Sequencer.
///
/// All prefixes are guaranteed to be between 1 and 83 bech32 human readable
/// characters in the ASCII range `[33, 126]`.
#[derive(Clone, Debug)]
pub struct AddressPrefixes {
    base: String,
    ibc_compat: String,
}

impl AddressPrefixes {
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    #[must_use]
    pub fn ibc_compat(&self) -> &str {
        &self.ibc_compat
    }
}

impl Protobuf for AddressPrefixes {
    type Error = AddressPrefixesError;
    type Raw = raw::AddressPrefixes;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        fn dummy_addr<T: crate::primitive::v1::Format>(prefix: &str) -> Result<(), AddressError> {
            Address::<T::Checksum>::builder()
                .array([0u8; crate::primitive::v1::ADDRESS_LEN])
                .prefix(prefix)
                .try_build()
                .map(|_| ())
        }

        let Self::Raw {
            base,
            ibc_compat,
        } = raw;

        dummy_addr::<Bech32m>(base).map_err(Self::Error::base)?;
        dummy_addr::<Bech32>(ibc_compat).map_err(Self::Error::base)?;

        Ok(Self {
            base: base.to_string(),
            ibc_compat: ibc_compat.to_string(),
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            base,
            ibc_compat,
        } = self;
        Self::Raw {
            base: base.clone(),
            ibc_compat: ibc_compat.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct AddressPrefixesError(AddressPrefixesErrorKind);

impl AddressPrefixesError {
    fn base(source: AddressError) -> Self {
        Self(AddressPrefixesErrorKind::Base {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed ensuring invariants of {}", AddressPrefixes::full_name())]
enum AddressPrefixesErrorKind {
    #[error("`base` cannot be used to construct Astria addresses")]
    Base { source: AddressError },
}

impl Protobuf for IBCParameters {
    type Error = Infallible;
    type Raw = raw::IbcParameters;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        Ok((*raw).into())
    }

    fn to_raw(&self) -> Self::Raw {
        self.clone().into()
    }
}

impl From<IBCParameters> for raw::IbcParameters {
    fn from(value: IBCParameters) -> Self {
        let IBCParameters {
            ibc_enabled,
            inbound_ics20_transfers_enabled,
            outbound_ics20_transfers_enabled,
        } = value;
        Self {
            ibc_enabled,
            inbound_ics20_transfers_enabled,
            outbound_ics20_transfers_enabled,
        }
    }
}

impl From<raw::IbcParameters> for IBCParameters {
    fn from(value: raw::IbcParameters) -> Self {
        let raw::IbcParameters {
            ibc_enabled,
            inbound_ics20_transfers_enabled,
            outbound_ics20_transfers_enabled,
        } = value;
        Self {
            ibc_enabled,
            inbound_ics20_transfers_enabled,
            outbound_ics20_transfers_enabled,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GenesisFees {
    pub rollup_data_submission: Option<FeeComponents<RollupDataSubmission>>,
    pub transfer: Option<FeeComponents<Transfer>>,
    pub ics20_withdrawal: Option<FeeComponents<Ics20Withdrawal>>,
    pub init_bridge_account: Option<FeeComponents<InitBridgeAccount>>,
    pub bridge_lock: Option<FeeComponents<BridgeLock>>,
    pub bridge_unlock: Option<FeeComponents<BridgeUnlock>>,
    pub bridge_sudo_change: Option<FeeComponents<BridgeSudoChange>>,
    pub ibc_relay: Option<FeeComponents<IbcRelay>>,
    pub validator_update: Option<FeeComponents<ValidatorUpdate>>,
    pub fee_asset_change: Option<FeeComponents<FeeAssetChange>>,
    pub fee_change: FeeComponents<FeeChange>,
    pub ibc_relayer_change: Option<FeeComponents<IbcRelayerChange>>,
    pub sudo_address_change: Option<FeeComponents<SudoAddressChange>>,
    pub ibc_sudo_change: Option<FeeComponents<IbcSudoChange>>,
    pub add_currency_pairs: Option<FeeComponents<AddCurrencyPairs>>,
    pub remove_currency_pairs: Option<FeeComponents<RemoveCurrencyPairs>>,
}

impl Protobuf for GenesisFees {
    type Error = FeesError;
    type Raw = raw::GenesisFees;

    #[expect(
        clippy::too_many_lines,
        reason = "barring use of a macro, all lines are necessary"
    )]
    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            rollup_data_submission,
            transfer,
            ics20_withdrawal,
            init_bridge_account,
            bridge_lock,
            bridge_unlock,
            bridge_sudo_change,
            ibc_relay,
            validator_update,
            fee_asset_change,
            fee_change,
            ibc_relayer_change,
            sudo_address_change,
            ibc_sudo_change,
            add_currency_pairs,
            remove_currency_pairs,
        } = raw;
        let rollup_data_submission = rollup_data_submission
            .clone()
            .map(FeeComponents::<RollupDataSubmission>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("rollup_data_submission", e))?;

        let transfer = transfer
            .clone()
            .map(FeeComponents::<Transfer>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("transfer", e))?;

        let ics20_withdrawal = ics20_withdrawal
            .clone()
            .map(FeeComponents::<Ics20Withdrawal>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("ics20_withdrawal", e))?;

        let init_bridge_account = init_bridge_account
            .clone()
            .map(FeeComponents::<InitBridgeAccount>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("init_bridge_account", e))?;

        let bridge_lock = bridge_lock
            .clone()
            .map(FeeComponents::<BridgeLock>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("bridge_lock", e))?;

        let bridge_unlock = bridge_unlock
            .clone()
            .map(FeeComponents::<BridgeUnlock>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("bridge_unlock", e))?;

        let bridge_sudo_change = bridge_sudo_change
            .clone()
            .map(FeeComponents::<BridgeSudoChange>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("bridge_sudo_change", e))?;

        let ibc_relay = ibc_relay
            .clone()
            .map(FeeComponents::<IbcRelay>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("ibc_relay", e))?;

        let validator_update = validator_update
            .clone()
            .map(FeeComponents::<ValidatorUpdate>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("validator_update", e))?;

        let fee_asset_change = fee_asset_change
            .clone()
            .map(FeeComponents::<FeeAssetChange>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("fee_asset_change", e))?;

        let fee_change = FeeComponents::<FeeChange>::try_from_raw(
            fee_change
                .clone()
                .ok_or_else(|| Self::Error::field_not_set("fee_change"))?,
        )
        .map_err(|e| FeesError::fee_components("fee_change", e))?;

        let ibc_relayer_change = ibc_relayer_change
            .clone()
            .map(FeeComponents::<IbcRelayerChange>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("ibc_relayer_change", e))?;

        let sudo_address_change = sudo_address_change
            .clone()
            .map(FeeComponents::<SudoAddressChange>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("sudo_address_change", e))?;

        let ibc_sudo_change = ibc_sudo_change
            .clone()
            .map(FeeComponents::<IbcSudoChange>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("ibc_sudo_change", e))?;

        let add_currency_pairs = add_currency_pairs
            .clone()
            .map(FeeComponents::<AddCurrencyPairs>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("add_currency_pairs", e))?;

        let remove_currency_pairs = remove_currency_pairs
            .clone()
            .map(FeeComponents::<RemoveCurrencyPairs>::try_from_raw)
            .transpose()
            .map_err(|e| FeesError::fee_components("remove_currency_pairs", e))?;

        Ok(Self {
            rollup_data_submission,
            transfer,
            ics20_withdrawal,
            init_bridge_account,
            bridge_lock,
            bridge_unlock,
            bridge_sudo_change,
            ibc_relay,
            validator_update,
            fee_asset_change,
            fee_change,
            ibc_relayer_change,
            sudo_address_change,
            ibc_sudo_change,
            add_currency_pairs,
            remove_currency_pairs,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            rollup_data_submission,
            transfer,
            ics20_withdrawal,
            init_bridge_account,
            bridge_lock,
            bridge_unlock,
            bridge_sudo_change,
            ibc_relay,
            validator_update,
            fee_asset_change,
            fee_change,
            ibc_relayer_change,
            sudo_address_change,
            ibc_sudo_change,
            add_currency_pairs,
            remove_currency_pairs,
        } = self;
        Self::Raw {
            transfer: transfer.map(|act| FeeComponents::<Transfer>::to_raw(&act)),
            rollup_data_submission: rollup_data_submission
                .map(|act| FeeComponents::<RollupDataSubmission>::to_raw(&act)),
            ics20_withdrawal: ics20_withdrawal
                .map(|act| FeeComponents::<Ics20Withdrawal>::to_raw(&act)),
            init_bridge_account: init_bridge_account
                .map(|act| FeeComponents::<InitBridgeAccount>::to_raw(&act)),
            bridge_lock: bridge_lock.map(|act| FeeComponents::<BridgeLock>::to_raw(&act)),
            bridge_unlock: bridge_unlock.map(|act| FeeComponents::<BridgeUnlock>::to_raw(&act)),
            bridge_sudo_change: bridge_sudo_change
                .map(|act| FeeComponents::<BridgeSudoChange>::to_raw(&act)),
            ibc_relay: ibc_relay.map(|act| FeeComponents::<IbcRelay>::to_raw(&act)),
            validator_update: validator_update
                .map(|act| FeeComponents::<ValidatorUpdate>::to_raw(&act)),
            fee_asset_change: fee_asset_change
                .map(|act| FeeComponents::<FeeAssetChange>::to_raw(&act)),
            fee_change: Some(fee_change.to_raw()),
            ibc_relayer_change: ibc_relayer_change
                .map(|act| FeeComponents::<IbcRelayerChange>::to_raw(&act)),
            sudo_address_change: sudo_address_change
                .map(|act| FeeComponents::<SudoAddressChange>::to_raw(&act)),
            ibc_sudo_change: ibc_sudo_change
                .map(|act| FeeComponents::<IbcSudoChange>::to_raw(&act)),
            add_currency_pairs: add_currency_pairs
                .map(|act| FeeComponents::<AddCurrencyPairs>::to_raw(&act)),
            remove_currency_pairs: remove_currency_pairs
                .map(|act| FeeComponents::<RemoveCurrencyPairs>::to_raw(&act)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FeesError(FeesErrorKind);

impl FeesError {
    fn field_not_set(name: &'static str) -> Self {
        Self(FeesErrorKind::FieldNotSet {
            name,
        })
    }

    fn fee_components(field: &'static str, err: FeeComponentError) -> Self {
        Self(FeesErrorKind::FeeComponentsConversion {
            field,
            source: err,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed ensuring invariants of {}", Fees::full_name())]
enum FeesErrorKind {
    #[error("field was not set: `{name}`")]
    FieldNotSet { name: &'static str },
    #[error("validating field `{field}` failed")]
    FeeComponentsConversion {
        field: &'static str,
        source: FeeComponentError,
    },
}

#[cfg(test)]
mod tests {
    use indexmap::indexmap;

    use super::*;
    use crate::{
        oracles::price_feed::{
            market_map::v2::{
                MarketMap,
                Params,
            },
            types::v2::CurrencyPairId,
        },
        primitive::v1::Address,
    };

    const ASTRIA_ADDRESS_PREFIX: &str = "astria";

    fn alice() -> Address {
        Address::builder()
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .slice(hex::decode("1c0c490f1b5528d8173c5de46d131160e4b2c0c3").unwrap())
            .try_build()
            .unwrap()
    }

    fn bob() -> Address {
        Address::builder()
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .slice(hex::decode("34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a").unwrap())
            .try_build()
            .unwrap()
    }

    fn charlie() -> Address {
        Address::builder()
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .slice(hex::decode("60709e2d391864b732b4f0f51e387abb76743871").unwrap())
            .try_build()
            .unwrap()
    }

    fn mallory() -> Address {
        Address::builder()
            .prefix("other")
            .slice(hex::decode("60709e2d391864b732b4f0f51e387abb76743871").unwrap())
            .try_build()
            .unwrap()
    }

    fn genesis_state_markets() -> MarketMap {
        use crate::oracles::price_feed::{
            market_map::v2::{
                Market,
                MarketMap,
                ProviderConfig,
                Ticker,
            },
            types::v2::CurrencyPair,
        };

        let markets = indexmap! {
            "ETH/USD".to_string() =>  Market {
                ticker: Ticker {
                    currency_pair: CurrencyPair::from_parts(
                        "ETH".parse().unwrap(),
                        "USD".parse().unwrap(),
                    ),
                    decimals: 8,
                    min_provider_count: 3,
                    enabled: true,
                    metadata_json: String::new(),
                },
                provider_configs: vec![ProviderConfig {
                    name: "coingecko_api".to_string(),
                    off_chain_ticker: "ethereum/usd".to_string(),
                    normalize_by_pair: Some(CurrencyPair::from_parts(
                        "USDT".parse().unwrap(),
                        "USD".parse().unwrap(),
                    )),
                    invert: false,
                    metadata_json: String::new(),
                }],
            },
        };

        MarketMap {
            markets,
        }
    }

    fn proto_genesis_state() -> raw::GenesisAppState {
        use crate::oracles::price_feed::{
            oracle::v2::{
                CurrencyPairGenesis,
                QuotePrice,
            },
            types::v2::{
                CurrencyPair,
                CurrencyPairNonce,
                Price,
            },
        };

        raw::GenesisAppState {
            accounts: vec![
                raw::Account {
                    address: Some(alice().to_raw()),
                    balance: Some(1_000_000_000_000_000_000.into()),
                },
                raw::Account {
                    address: Some(bob().to_raw()),
                    balance: Some(1_000_000_000_000_000_000.into()),
                },
                raw::Account {
                    address: Some(charlie().to_raw()),
                    balance: Some(1_000_000_000_000_000_000.into()),
                },
            ],
            address_prefixes: Some(raw::AddressPrefixes {
                base: "astria".into(),
                ibc_compat: "astriacompat".into(),
            }),
            authority_sudo_address: Some(alice().to_raw()),
            chain_id: "astria-1".to_string(),
            ibc_sudo_address: Some(alice().to_raw()),
            ibc_relayer_addresses: vec![alice().to_raw(), bob().to_raw()],
            native_asset_base_denomination: "nria".to_string(),
            ibc_parameters: Some(raw::IbcParameters {
                ibc_enabled: true,
                inbound_ics20_transfers_enabled: true,
                outbound_ics20_transfers_enabled: true,
            }),
            allowed_fee_assets: vec!["nria".into()],
            fees: Some(raw::GenesisFees {
                transfer: Some(FeeComponents::<Transfer>::new(12, 0).to_raw()),
                rollup_data_submission: Some(
                    FeeComponents::<RollupDataSubmission>::new(32, 1).to_raw(),
                ),
                init_bridge_account: Some(FeeComponents::<InitBridgeAccount>::new(48, 0).to_raw()),
                bridge_lock: Some(FeeComponents::<BridgeLock>::new(12, 1).to_raw()),
                bridge_unlock: Some(FeeComponents::<BridgeUnlock>::new(12, 0).to_raw()),
                bridge_sudo_change: Some(FeeComponents::<BridgeSudoChange>::new(24, 0).to_raw()),
                ics20_withdrawal: Some(FeeComponents::<Ics20Withdrawal>::new(24, 0).to_raw()),
                ibc_relay: Some(FeeComponents::<IbcRelay>::new(0, 0).to_raw()),
                validator_update: Some(FeeComponents::<ValidatorUpdate>::new(0, 0).to_raw()),
                fee_asset_change: Some(FeeComponents::<FeeAssetChange>::new(0, 0).to_raw()),
                fee_change: Some(FeeComponents::<FeeChange>::new(0, 0).to_raw()),
                ibc_relayer_change: Some(FeeComponents::<IbcRelayerChange>::new(0, 0).to_raw()),
                sudo_address_change: Some(FeeComponents::<SudoAddressChange>::new(0, 0).to_raw()),
                ibc_sudo_change: Some(FeeComponents::<IbcSudoChange>::new(0, 0).to_raw()),
                add_currency_pairs: Some(FeeComponents::<AddCurrencyPairs>::new(0, 0).to_raw()),
                remove_currency_pairs: Some(
                    FeeComponents::<RemoveCurrencyPairs>::new(0, 0).to_raw(),
                ),
            }),
            price_feed: Some(
                PriceFeedGenesis {
                    market_map: market_map::v2::GenesisState {
                        market_map: genesis_state_markets(),
                        last_updated: 0,
                        params: Params {
                            market_authorities: vec![alice(), bob()],
                            admin: alice(),
                        },
                    },
                    oracle: oracle::v2::GenesisState {
                        currency_pair_genesis: vec![CurrencyPairGenesis {
                            id: CurrencyPairId::new(1),
                            nonce: CurrencyPairNonce::new(0),
                            currency_pair_price: Some(QuotePrice {
                                price: Price::new(3_138_872_234_u128),
                                block_height: 0,
                                block_timestamp: pbjson_types::Timestamp {
                                    seconds: 1_720_122_395,
                                    nanos: 0,
                                },
                            }),
                            currency_pair: CurrencyPair::from_parts(
                                "ETH".parse().unwrap(),
                                "USD".parse().unwrap(),
                            ),
                        }],
                        next_id: CurrencyPairId::new(1),
                    },
                }
                .into_raw(),
            ),
        }
    }

    fn genesis_state() -> GenesisAppState {
        proto_genesis_state().try_into().unwrap()
    }

    #[test]
    fn mismatched_addresses_are_caught() {
        #[track_caller]
        fn assert_bad_prefix(unchecked: raw::GenesisAppState, bad_field: &'static str) {
            match GenesisAppState::try_from(unchecked)
                .expect_err(
                    "converting to genesis state should have produced an error, but a valid state \
                     was returned",
                )
                .0
            {
                GenesisAppStateErrorKind::AddressDoesNotMatchBase {
                    source,
                } => {
                    let AddressDoesNotMatchBase {
                        base_prefix,
                        address,
                        field,
                    } = *source;
                    assert_eq!(base_prefix, ASTRIA_ADDRESS_PREFIX);
                    assert_eq!(address, mallory());
                    assert_eq!(field, bad_field);
                }
                other => panic!(
                    "expected: `GenesisAppStateErrorKind::AddressDoesNotMatchBase\ngot: {other:?}`"
                ),
            };
        }
        assert_bad_prefix(
            raw::GenesisAppState {
                authority_sudo_address: Some(mallory().to_raw()),
                ..proto_genesis_state()
            },
            ".authority_sudo_address",
        );
        assert_bad_prefix(
            raw::GenesisAppState {
                ibc_sudo_address: Some(mallory().to_raw()),
                ..proto_genesis_state()
            },
            ".ibc_sudo_address",
        );
        assert_bad_prefix(
            raw::GenesisAppState {
                ibc_relayer_addresses: vec![alice().to_raw(), mallory().to_raw()],
                ..proto_genesis_state()
            },
            ".ibc_relayer_addresses[1]",
        );
        assert_bad_prefix(
            raw::GenesisAppState {
                price_feed: {
                    let mut price_feed = proto_genesis_state().price_feed;
                    price_feed
                        .as_mut()
                        .unwrap()
                        .market_map
                        .as_mut()
                        .unwrap()
                        .params
                        .as_mut()
                        .unwrap()
                        .market_authorities[0] = mallory().to_string();
                    price_feed
                },
                ..proto_genesis_state()
            },
            ".market_map.params.market_authorities[0]",
        );
        assert_bad_prefix(
            raw::GenesisAppState {
                accounts: vec![
                    raw::Account {
                        address: Some(alice().to_raw()),
                        balance: Some(10.into()),
                    },
                    raw::Account {
                        address: Some(mallory().to_raw()),
                        balance: Some(10.into()),
                    },
                ],
                ..proto_genesis_state()
            },
            ".accounts[1].address",
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn genesis_state_is_unchanged() {
        insta::assert_json_snapshot!("genesis_state", genesis_state());
    }
}
