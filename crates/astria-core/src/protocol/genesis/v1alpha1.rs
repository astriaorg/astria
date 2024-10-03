use std::convert::Infallible;

pub use penumbra_ibc::params::IBCParameters;

use crate::{
    generated::protocol::genesis::v1alpha1 as raw,
    primitive::v1::{
        asset::{
            self,
            denom::ParseTracePrefixedError,
            ParseDenomError,
        },
        try_construct_dummy_address_from_prefix,
        Address,
        AddressError,
        Bech32,
        Bech32m,
    },
    slinky::{
        market_map,
        oracle,
    },
    Protobuf,
};

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(try_from = "raw::SlinkyGenesis", into = "raw::SlinkyGenesis")
)]
pub struct SlinkyGenesis {
    market_map: market_map::v1::GenesisState,
    oracle: oracle::v1::GenesisState,
}

impl SlinkyGenesis {
    #[must_use]
    pub fn market_map(&self) -> &market_map::v1::GenesisState {
        &self.market_map
    }

    #[must_use]
    pub fn oracle(&self) -> &oracle::v1::GenesisState {
        &self.oracle
    }
}

impl Protobuf for SlinkyGenesis {
    type Error = SlinkyGenesisError;
    type Raw = raw::SlinkyGenesis;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            market_map,
            oracle,
        } = raw;
        let market_map = market_map
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("market_map"))
            .and_then(|market_map| {
                market_map::v1::GenesisState::try_from_raw_ref(market_map)
                    .map_err(Self::Error::market_map)
            })?;
        let oracle = oracle
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("oracle"))
            .and_then(|oracle| {
                oracle::v1::GenesisState::try_from_raw_ref(oracle).map_err(Self::Error::oracle)
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

impl TryFrom<raw::SlinkyGenesis> for SlinkyGenesis {
    type Error = <Self as Protobuf>::Error;

    fn try_from(value: raw::SlinkyGenesis) -> Result<Self, Self::Error> {
        Self::try_from_raw(value)
    }
}

impl From<SlinkyGenesis> for raw::SlinkyGenesis {
    fn from(value: SlinkyGenesis) -> Self {
        value.into_raw()
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SlinkyGenesisError(SlinkyGenesisErrorKind);

impl SlinkyGenesisError {
    fn field_not_set(name: &'static str) -> Self {
        Self(SlinkyGenesisErrorKind::FieldNotSet {
            name,
        })
    }

    fn market_map(source: market_map::v1::GenesisStateError) -> Self {
        Self(SlinkyGenesisErrorKind::MarketMap {
            source,
        })
    }

    fn oracle(source: oracle::v1::GenesisStateError) -> Self {
        Self(SlinkyGenesisErrorKind::Oracle {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed ensuring invariants of {}", SlinkyGenesis::full_name())]
enum SlinkyGenesisErrorKind {
    #[error("field was not set: `{name}`")]
    FieldNotSet { name: &'static str },
    #[error("`market_map` field was invalid")]
    MarketMap {
        source: market_map::v1::GenesisStateError,
    },
    #[error("`oracle` field was invalid")]
    Oracle {
        source: oracle::v1::GenesisStateError,
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
    native_asset_base_denomination: asset::TracePrefixed,
    ibc_parameters: IBCParameters,
    allowed_fee_assets: Vec<asset::Denom>,
    fees: Fees,
    slinky: SlinkyGenesis,
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
    pub fn native_asset_base_denomination(&self) -> &asset::TracePrefixed {
        &self.native_asset_base_denomination
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
    pub fn fees(&self) -> &Fees {
        &self.fees
    }

    #[must_use]
    pub fn slinky(&self) -> &SlinkyGenesis {
        &self.slinky
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

        for (i, address) in self
            .slinky
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
            &self.slinky.market_map.params.admin,
            ".market_map.params.admin",
        )?;

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
            slinky,
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
                Address::try_from_raw(addr).map_err(Self::Error::authority_sudo_address)
            })?;
        let ibc_sudo_address = ibc_sudo_address
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("ibc_sudo_address"))
            .and_then(|addr| Address::try_from_raw(addr).map_err(Self::Error::ibc_sudo_address))?;

        let ibc_relayer_addresses = ibc_relayer_addresses
            .iter()
            .map(Address::try_from_raw)
            .collect::<Result<_, _>>()
            .map_err(Self::Error::ibc_relayer_addresses)?;

        let native_asset_base_denomination = native_asset_base_denomination
            .parse()
            .map_err(Self::Error::native_asset_base_denomination)?;

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
            .and_then(|fees| Fees::try_from_raw_ref(fees).map_err(Self::Error::fees))?;

        let slinky = slinky
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("slinky"))?;

        let market_map = slinky
            .market_map
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("market_map"))
            .and_then(|market_map| {
                market_map::v1::GenesisState::try_from_raw(market_map.clone())
                    .map_err(Self::Error::market_map)
            })?;

        let oracle = slinky
            .oracle
            .as_ref()
            .ok_or_else(|| Self::Error::field_not_set("oracle"))
            .and_then(|oracle| {
                oracle::v1::GenesisState::try_from_raw(oracle.clone()).map_err(Self::Error::oracle)
            })?;

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
            slinky: SlinkyGenesis {
                market_map,
                oracle,
            },
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
            slinky,
        } = self;
        Self::Raw {
            address_prefixes: Some(address_prefixes.to_raw()),
            accounts: accounts.iter().map(Account::to_raw).collect(),
            authority_sudo_address: Some(authority_sudo_address.to_raw()),
            chain_id: chain_id.clone(),
            ibc_sudo_address: Some(ibc_sudo_address.to_raw()),
            ibc_relayer_addresses: ibc_relayer_addresses.iter().map(Address::to_raw).collect(),
            native_asset_base_denomination: native_asset_base_denomination.to_string(),
            ibc_parameters: Some(ibc_parameters.to_raw()),
            allowed_fee_assets: allowed_fee_assets.iter().map(ToString::to_string).collect(),
            fees: Some(fees.to_raw()),
            slinky: Some(slinky.to_raw()),
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

    fn market_map(source: market_map::v1::GenesisStateError) -> Self {
        Self(GenesisAppStateErrorKind::MarketMap {
            source,
        })
    }

    fn oracle(source: oracle::v1::GenesisStateError) -> Self {
        Self(GenesisAppStateErrorKind::Oracle {
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
    #[error("`market_map` field was invalid")]
    MarketMap {
        source: market_map::v1::GenesisStateError,
    },
    #[error("`oracle` field was invalid")]
    Oracle {
        source: oracle::v1::GenesisStateError,
    },
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
            .and_then(|addr| Address::try_from_raw(addr).map_err(Self::Error::address))?;
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
        let Self::Raw {
            base,
            ibc_compat,
        } = raw;
        try_construct_dummy_address_from_prefix::<Bech32m>(base).map_err(Self::Error::base)?;
        try_construct_dummy_address_from_prefix::<Bech32>(ibc_compat).map_err(Self::Error::base)?;
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
pub struct Fees {
    pub transfer_base_fee: u128,
    pub sequence_base_fee: u128,
    pub sequence_byte_cost_multiplier: u128,
    pub init_bridge_account_base_fee: u128,
    pub bridge_lock_byte_cost_multiplier: u128,
    pub bridge_sudo_change_fee: u128,
    pub ics20_withdrawal_base_fee: u128,
}

impl Protobuf for Fees {
    type Error = FeesError;
    type Raw = raw::Fees;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            transfer_base_fee,
            sequence_base_fee,
            sequence_byte_cost_multiplier,
            init_bridge_account_base_fee,
            bridge_lock_byte_cost_multiplier,
            bridge_sudo_change_fee,
            ics20_withdrawal_base_fee,
        } = raw;
        let transfer_base_fee = transfer_base_fee
            .ok_or_else(|| Self::Error::field_not_set("transfer_base_fee"))?
            .into();
        let sequence_base_fee = sequence_base_fee
            .ok_or_else(|| Self::Error::field_not_set("sequence_base_fee"))?
            .into();
        let sequence_byte_cost_multiplier = sequence_byte_cost_multiplier
            .ok_or_else(|| Self::Error::field_not_set("sequence_byte_cost_multiplier"))?
            .into();
        let init_bridge_account_base_fee = init_bridge_account_base_fee
            .ok_or_else(|| Self::Error::field_not_set("init_bridge_account_base_fee"))?
            .into();
        let bridge_lock_byte_cost_multiplier = bridge_lock_byte_cost_multiplier
            .ok_or_else(|| Self::Error::field_not_set("bridge_lock_byte_cost_multiplier"))?
            .into();
        let bridge_sudo_change_fee = bridge_sudo_change_fee
            .ok_or_else(|| Self::Error::field_not_set("bridge_sudo_change_fee"))?
            .into();
        let ics20_withdrawal_base_fee = ics20_withdrawal_base_fee
            .ok_or_else(|| Self::Error::field_not_set("ics20_withdrawal_base_fee"))?
            .into();
        Ok(Self {
            transfer_base_fee,
            sequence_base_fee,
            sequence_byte_cost_multiplier,
            init_bridge_account_base_fee,
            bridge_lock_byte_cost_multiplier,
            bridge_sudo_change_fee,
            ics20_withdrawal_base_fee,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            transfer_base_fee,
            sequence_base_fee,
            sequence_byte_cost_multiplier,
            init_bridge_account_base_fee,
            bridge_lock_byte_cost_multiplier,
            bridge_sudo_change_fee,
            ics20_withdrawal_base_fee,
        } = self;
        Self::Raw {
            transfer_base_fee: Some(transfer_base_fee.into()),
            sequence_base_fee: Some(sequence_base_fee.into()),
            sequence_byte_cost_multiplier: Some(sequence_byte_cost_multiplier.into()),
            init_bridge_account_base_fee: Some(init_bridge_account_base_fee.into()),
            bridge_lock_byte_cost_multiplier: Some(bridge_lock_byte_cost_multiplier.into()),
            bridge_sudo_change_fee: Some(bridge_sudo_change_fee.into()),
            ics20_withdrawal_base_fee: Some(ics20_withdrawal_base_fee.into()),
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
}

#[derive(Debug, thiserror::Error)]
#[error("failed ensuring invariants of {}", Fees::full_name())]
enum FeesErrorKind {
    #[error("field was not set: `{name}`")]
    FieldNotSet { name: &'static str },
}

#[cfg(test)]
mod tests {
    use indexmap::indexmap;

    use super::*;
    use crate::{
        primitive::v1::Address,
        slinky::{
            market_map::v1::{
                MarketMap,
                Params,
            },
            types::v1::CurrencyPairId,
        },
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
        use crate::slinky::{
            market_map::v1::{
                Market,
                MarketMap,
                ProviderConfig,
                Ticker,
            },
            types::v1::CurrencyPair,
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
                    normalize_by_pair: CurrencyPair::from_parts(
                        "USDT".parse().unwrap(),
                        "USD".parse().unwrap(),
                    ),
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
        use crate::slinky::{
            oracle::v1::{
                CurrencyPairGenesis,
                QuotePrice,
            },
            types::v1::{
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
            fees: Some(raw::Fees {
                transfer_base_fee: Some(12.into()),
                sequence_base_fee: Some(32.into()),
                sequence_byte_cost_multiplier: Some(1.into()),
                init_bridge_account_base_fee: Some(48.into()),
                bridge_lock_byte_cost_multiplier: Some(1.into()),
                bridge_sudo_change_fee: Some(24.into()),
                ics20_withdrawal_base_fee: Some(24.into()),
            }),
            slinky: Some(
                SlinkyGenesis {
                    market_map: market_map::v1::GenesisState {
                        market_map: genesis_state_markets(),
                        last_updated: 0,
                        params: Params {
                            market_authorities: vec![alice(), bob()],
                            admin: alice(),
                        },
                    },
                    oracle: oracle::v1::GenesisState {
                        currency_pair_genesis: vec![CurrencyPairGenesis {
                            id: CurrencyPairId::new(1),
                            nonce: CurrencyPairNonce::new(0),
                            currency_pair_price: QuotePrice {
                                price: Price::new(3_138_872_234_u128),
                                block_height: 0,
                                block_timestamp: pbjson_types::Timestamp {
                                    seconds: 1_720_122_395,
                                    nanos: 0,
                                },
                            },
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
                slinky: {
                    let mut slinky = proto_genesis_state().slinky;
                    slinky
                        .as_mut()
                        .unwrap()
                        .market_map
                        .as_mut()
                        .unwrap()
                        .params
                        .as_mut()
                        .unwrap()
                        .market_authorities[0] = mallory().to_string();
                    slinky
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
        insta::assert_json_snapshot!(genesis_state());
    }
}
