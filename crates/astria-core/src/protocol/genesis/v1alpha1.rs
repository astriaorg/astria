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
        Address,
        AddressError,
        ADDRESS_LEN,
    },
    Protobuf,
};

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
    address_prefixes: AddressPrefixes,
    accounts: Vec<Account>,
    authority_sudo_address: crate::primitive::v1::Address,
    ibc_sudo_address: crate::primitive::v1::Address,
    ibc_relayer_addresses: Vec<crate::primitive::v1::Address>,
    native_asset_base_denomination: asset::TracePrefixed,
    ibc_parameters: IBCParameters,
    allowed_fee_assets: Vec<asset::Denom>,
}

impl GenesisAppState {
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
        Ok(())
    }
}

impl Protobuf for GenesisAppState {
    type Error = GenesisAppStateError;
    type Raw = raw::GenesisAppState;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            address_prefixes,
            accounts,
            authority_sudo_address,
            ibc_sudo_address,
            ibc_relayer_addresses,
            native_asset_base_denomination,
            ibc_parameters,
            allowed_fee_assets,
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

        let this = Self {
            address_prefixes,
            accounts,
            authority_sudo_address,
            ibc_sudo_address,
            ibc_relayer_addresses,
            native_asset_base_denomination,
            ibc_parameters,
            allowed_fee_assets,
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
            ibc_sudo_address,
            ibc_relayer_addresses,
            native_asset_base_denomination,
            ibc_parameters,
            allowed_fee_assets,
        } = self;
        Self::Raw {
            address_prefixes: Some(address_prefixes.to_raw()),
            accounts: accounts.iter().map(Account::to_raw).collect(),
            authority_sudo_address: Some(authority_sudo_address.to_raw()),
            ibc_sudo_address: Some(ibc_sudo_address.to_raw()),
            ibc_relayer_addresses: ibc_relayer_addresses.iter().map(Address::to_raw).collect(),
            native_asset_base_denomination: native_asset_base_denomination.to_string(),
            ibc_parameters: Some(ibc_parameters.to_raw()),
            allowed_fee_assets: allowed_fee_assets.iter().map(ToString::to_string).collect(),
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
}

#[derive(Debug, thiserror::Error)]
#[error("failed ensuring invariants of {}", GenesisAppState::full_name())]
enum GenesisAppStateErrorKind {
    #[error("`accounts` field was invalid")]
    Accounts { source: AccountError },
    #[error("`address_prefixes` field was invalid")]
    AddressPrefixes { source: AddressPrefixesError },
    #[error("one or more addresses did not match the provided base prefix")]
    AddressDoesNotMatchBase {
        source: Box<AddressDoesNotMatchBase>,
    },
    #[error("`allowed_fee_assets` field was invalid")]
    AllowedFeeAssets { source: ParseDenomError },
    #[error("`authority_sudo_address` field was invalid")]
    AuthoritySudoAddress { source: AddressError },
    #[error("`ibc_sudo_address` field was invalid")]
    IbcSudoAddress { source: AddressError },
    #[error("`ibc_relayer_addresses` field was invalid")]
    IbcRelayerAddresses { source: AddressError },
    #[error("field was not set: `{name}`")]
    FieldNotSet { name: &'static str },
    #[error("`native_asset_base_denomination` field was invalid")]
    NativeAssetBaseDenomination { source: ParseTracePrefixedError },
}

#[derive(Debug, thiserror::Error)]
#[error("address `{address}` at `{field}` does not have `{base_prefix}`")]
struct AddressDoesNotMatchBase {
    base_prefix: String,
    address: Address,
    field: String,
}

#[derive(Clone, Debug)]
pub struct Account {
    address: Address,
    balance: u128,
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

#[derive(Clone, Debug)]
pub struct AddressPrefixes {
    base: String,
}

impl Protobuf for AddressPrefixes {
    type Error = AddressPrefixesError;
    type Raw = raw::AddressPrefixes;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            base,
        } = raw;
        try_construct_dummy_address_from_prefix(base).map_err(Self::Error::base)?;
        Ok(Self {
            base: base.to_string(),
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            base,
        } = self;
        Self::Raw {
            base: base.clone(),
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
        let Self::Raw {
            ibc_enabled,
            inbound_ics20_transfers_enabled,
            outbound_ics20_transfers_enabled,
        } = *raw;
        Ok(Self {
            ibc_enabled,
            inbound_ics20_transfers_enabled,
            outbound_ics20_transfers_enabled,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            ibc_enabled,
            inbound_ics20_transfers_enabled,
            outbound_ics20_transfers_enabled,
        } = self;
        Self::Raw {
            ibc_enabled: *ibc_enabled,
            inbound_ics20_transfers_enabled: *inbound_ics20_transfers_enabled,
            outbound_ics20_transfers_enabled: *outbound_ics20_transfers_enabled,
        }
    }
}

/// Constructs a dummy address from a given `prefix`, otherwise fail.
fn try_construct_dummy_address_from_prefix(prefix: &str) -> Result<(), AddressError> {
    Address::builder()
        .array([0u8; ADDRESS_LEN])
        .prefix(prefix)
        .try_build()
        .map(|_| ())
}
