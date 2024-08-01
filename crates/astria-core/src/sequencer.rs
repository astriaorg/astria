//! Sequencer specific types that are needed outside of it.
pub use penumbra_ibc::params::IBCParameters;

use crate::primitive::v1::{
    asset::{
        self,
        TracePrefixed,
    },
    Address,
};

/// The genesis state of Astria's Sequencer.
///
/// Verified to only contain valid fields (right now, addresses that have the same base prefix
/// as set in `GenesisState::address_prefixes::base`).
///
/// *Note on the implementation:* access to all fields is through getters to uphold invariants,
/// but most returned values themselves have publicly exposed fields. This is to make it easier
/// to construct an [`UncheckedGenesisState`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(try_from = "UncheckedGenesisState", into = "UncheckedGenesisState")
)]
pub struct GenesisState {
    address_prefixes: AddressPrefixes,
    accounts: Vec<Account>,
    authority_sudo_address: Address,
    ibc_sudo_address: Address,
    ibc_relayer_addresses: Vec<Address>,
    native_asset_base_denomination: TracePrefixed,
    ibc_params: IBCParameters,
    allowed_fee_assets: Vec<asset::Denom>,
    fees: Fees,
}

impl GenesisState {
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
    pub fn ibc_sudo_address(&self) -> &Address {
        &self.ibc_sudo_address
    }

    #[must_use]
    pub fn ibc_relayer_addresses(&self) -> &[Address] {
        &self.ibc_relayer_addresses
    }

    #[must_use]
    pub fn native_asset_base_denomination(&self) -> &TracePrefixed {
        &self.native_asset_base_denomination
    }

    #[must_use]
    pub fn ibc_params(&self) -> &IBCParameters {
        &self.ibc_params
    }

    #[must_use]
    pub fn allowed_fee_assets(&self) -> &[asset::Denom] {
        &self.allowed_fee_assets
    }

    #[must_use]
    pub fn fees(&self) -> &Fees {
        &self.fees
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct VerifyGenesisError(Box<VerifyGenesisErrorKind>);

#[derive(Debug, thiserror::Error)]
enum VerifyGenesisErrorKind {
    #[error("address `{address}` at `{field}` does not have `{base_prefix}`")]
    AddressDoesNotMatchBase {
        base_prefix: String,
        address: Address,
        field: String,
    },
}

impl From<VerifyGenesisErrorKind> for VerifyGenesisError {
    fn from(value: VerifyGenesisErrorKind) -> Self {
        Self(Box::new(value))
    }
}

impl TryFrom<UncheckedGenesisState> for GenesisState {
    type Error = VerifyGenesisError;

    fn try_from(value: UncheckedGenesisState) -> Result<Self, Self::Error> {
        value.ensure_all_addresses_have_base_prefix()?;

        let UncheckedGenesisState {
            address_prefixes,
            accounts,
            authority_sudo_address,
            ibc_sudo_address,
            ibc_relayer_addresses,
            native_asset_base_denomination,
            ibc_params,
            allowed_fee_assets,
            fees,
        } = value;

        Ok(Self {
            address_prefixes,
            accounts,
            authority_sudo_address,
            ibc_sudo_address,
            ibc_relayer_addresses,
            native_asset_base_denomination,
            ibc_params,
            allowed_fee_assets,
            fees,
        })
    }
}

/// The unchecked genesis state for the application.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct UncheckedGenesisState {
    pub address_prefixes: AddressPrefixes,
    pub accounts: Vec<Account>,
    pub authority_sudo_address: Address,
    pub ibc_sudo_address: Address,
    pub ibc_relayer_addresses: Vec<Address>,
    pub native_asset_base_denomination: TracePrefixed,
    pub ibc_params: IBCParameters,
    pub allowed_fee_assets: Vec<asset::Denom>,
    pub fees: Fees,
}

impl UncheckedGenesisState {
    fn ensure_address_has_base_prefix(
        &self,
        address: &Address,
        field: &str,
    ) -> Result<(), VerifyGenesisError> {
        if self.address_prefixes.base != address.prefix() {
            return Err(VerifyGenesisErrorKind::AddressDoesNotMatchBase {
                base_prefix: self.address_prefixes.base.clone(),
                address: *address,
                field: field.to_string(),
            }
            .into());
        }
        Ok(())
    }

    fn ensure_all_addresses_have_base_prefix(&self) -> Result<(), VerifyGenesisError> {
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

impl From<GenesisState> for UncheckedGenesisState {
    fn from(value: GenesisState) -> Self {
        let GenesisState {
            address_prefixes,
            accounts,
            authority_sudo_address,
            ibc_sudo_address,
            ibc_relayer_addresses,
            native_asset_base_denomination,
            ibc_params,
            allowed_fee_assets,
            fees,
        } = value;
        Self {
            address_prefixes,
            accounts,
            authority_sudo_address,
            ibc_sudo_address,
            ibc_relayer_addresses,
            native_asset_base_denomination,
            ibc_params,
            allowed_fee_assets,
            fees,
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Fees {
    pub transfer_base_fee: u128,
    pub sequence_base_fee: u128,
    pub sequence_byte_cost_multiplier: u128,
    pub init_bridge_account_base_fee: u128,
    pub bridge_lock_byte_cost_multiplier: u128,
    pub bridge_sudo_change_fee: u128,
    pub ics20_withdrawal_base_fee: u128,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Account {
    pub address: Address,
    pub balance: u128,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AddressPrefixes {
    pub base: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive::v1::Address;

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

    fn unchecked_genesis_state() -> UncheckedGenesisState {
        UncheckedGenesisState {
            accounts: vec![
                Account {
                    address: alice(),
                    balance: 1_000_000_000_000_000_000,
                },
                Account {
                    address: bob(),
                    balance: 1_000_000_000_000_000_000,
                },
                Account {
                    address: charlie(),
                    balance: 1_000_000_000_000_000_000,
                },
            ],
            address_prefixes: AddressPrefixes {
                base: "astria".into(),
            },
            authority_sudo_address: alice(),
            ibc_sudo_address: alice(),
            ibc_relayer_addresses: vec![alice(), bob()],
            native_asset_base_denomination: "nria".parse().unwrap(),
            ibc_params: IBCParameters {
                ibc_enabled: true,
                inbound_ics20_transfers_enabled: true,
                outbound_ics20_transfers_enabled: true,
            },
            allowed_fee_assets: vec!["nria".parse().unwrap()],
            fees: Fees {
                transfer_base_fee: 12,
                sequence_base_fee: 32,
                sequence_byte_cost_multiplier: 1,
                init_bridge_account_base_fee: 48,
                bridge_lock_byte_cost_multiplier: 1,
                bridge_sudo_change_fee: 24,
                ics20_withdrawal_base_fee: 24,
            },
        }
    }

    fn genesis_state() -> GenesisState {
        unchecked_genesis_state().try_into().unwrap()
    }

    #[test]
    fn mismatched_addresses_are_caught() {
        #[track_caller]
        fn assert_bad_prefix(unchecked: UncheckedGenesisState, bad_field: &'static str) {
            match *GenesisState::try_from(unchecked)
                .expect_err(
                    "converting to genesis state should have produced an error, but a valid state \
                     was returned",
                )
                .0
            {
                VerifyGenesisErrorKind::AddressDoesNotMatchBase {
                    base_prefix,
                    address,
                    field,
                } => {
                    assert_eq!(base_prefix, ASTRIA_ADDRESS_PREFIX);
                    assert_eq!(address, mallory());
                    assert_eq!(field, bad_field);
                }
            };
        }
        assert_bad_prefix(
            UncheckedGenesisState {
                authority_sudo_address: mallory(),
                ..unchecked_genesis_state()
            },
            ".authority_sudo_address",
        );
        assert_bad_prefix(
            UncheckedGenesisState {
                ibc_sudo_address: mallory(),
                ..unchecked_genesis_state()
            },
            ".ibc_sudo_address",
        );
        assert_bad_prefix(
            UncheckedGenesisState {
                ibc_relayer_addresses: vec![alice(), mallory()],
                ..unchecked_genesis_state()
            },
            ".ibc_relayer_addresses[1]",
        );
        assert_bad_prefix(
            UncheckedGenesisState {
                accounts: vec![
                    Account {
                        address: alice(),
                        balance: 10,
                    },
                    Account {
                        address: mallory(),
                        balance: 10,
                    },
                ],
                ..unchecked_genesis_state()
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
