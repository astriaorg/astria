use astria_core::primitive::v1::{
    asset,
    Address,
};
use penumbra_ibc::params::IBCParameters;
use serde::{
    Deserialize,
    Serialize,
};

/// The genesis state for the application.
///
/// Verified to only contain valid fields (right now, addresses that have the same base prefix
/// as set in `GenesisState::address_prefixes::base`).
///
/// **NOTE:** The fields should not be publicly accessible to guarantee invariants. However,
/// it's easy to just go along with this for now.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "UncheckedGenesisState", into = "UncheckedGenesisState")]
pub(crate) struct GenesisState {
    pub(crate) address_prefixes: AddressPrefixes,
    pub(crate) accounts: Vec<Account>,
    pub(crate) authority_sudo_address: Address,
    pub(crate) ibc_sudo_address: Address,
    pub(crate) ibc_relayer_addresses: Vec<Address>,
    pub(crate) native_asset_base_denomination: String,
    pub(crate) ibc_params: IBCParameters,
    pub(crate) allowed_fee_assets: Vec<asset::Denom>,
    pub(crate) fees: Fees,
}

#[derive(Debug, thiserror::Error)]
// allow: this error is only seen at chain init and never after so perf impact of too large enum
// variants is negligible
#[allow(clippy::result_large_err)]
pub(crate) enum VerifyGenesisError {
    #[error("address `{address}` at `{field}` does not have `{base_prefix}`")]
    AddressDoesNotMatchBase {
        base_prefix: String,
        address: Address,
        field: String,
    },
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
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UncheckedGenesisState {
    pub(crate) address_prefixes: AddressPrefixes,
    pub(crate) accounts: Vec<Account>,
    pub(crate) authority_sudo_address: Address,
    pub(crate) ibc_sudo_address: Address,
    pub(crate) ibc_relayer_addresses: Vec<Address>,
    pub(crate) native_asset_base_denomination: String,
    pub(crate) ibc_params: IBCParameters,
    pub(crate) allowed_fee_assets: Vec<asset::Denom>,
    pub(crate) fees: Fees,
}

impl UncheckedGenesisState {
    // allow: as for the enum definition itself: this only happens at init-chain and is negligible
    #[allow(clippy::result_large_err)]
    fn ensure_address_has_base_prefix(
        &self,
        address: &Address,
        field: &str,
    ) -> Result<(), VerifyGenesisError> {
        if self.address_prefixes.base != address.prefix() {
            return Err(VerifyGenesisError::AddressDoesNotMatchBase {
                base_prefix: self.address_prefixes.base.clone(),
                address: *address,
                field: field.to_string(),
            });
        }
        Ok(())
    }

    // allow: as for the enum definition itself: this only happens at init-chain and is negligible
    #[allow(clippy::result_large_err)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Fees {
    pub(crate) transfer_base_fee: u128,
    pub(crate) sequence_base_fee: u128,
    pub(crate) sequence_byte_cost_multiplier: u128,
    pub(crate) init_bridge_account_base_fee: u128,
    pub(crate) bridge_lock_byte_cost_multiplier: u128,
    pub(crate) bridge_sudo_change_fee: u128,
    pub(crate) ics20_withdrawal_base_fee: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Account {
    pub(crate) address: Address,
    pub(crate) balance: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AddressPrefixes {
    pub(crate) base: String,
}

#[cfg(test)]
mod test {
    use astria_core::primitive::v1::Address;

    use super::*;

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
            native_asset_base_denomination: "nria".to_string(),
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
            match GenesisState::try_from(unchecked).expect_err(
                "converting to genesis state should have produced an error, but a valid state was \
                 returned",
            ) {
                VerifyGenesisError::AddressDoesNotMatchBase {
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

    #[test]
    fn genesis_state_is_unchanged() {
        insta::assert_json_snapshot!(genesis_state());
    }
}
