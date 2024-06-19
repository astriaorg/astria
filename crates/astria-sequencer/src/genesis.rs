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
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct GenesisState {
    pub(crate) accounts: Vec<Account>,
    pub(crate) authority_sudo_address: Address,
    pub(crate) ibc_sudo_address: Address,
    pub(crate) ibc_relayer_addresses: Vec<Address>,
    pub(crate) native_asset_base_denomination: String,
    pub(crate) ibc_params: IBCParameters,
    pub(crate) allowed_fee_assets: Vec<asset::Denom>,
    pub(crate) fees: Fees,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Fees {
    pub(crate) transfer_base_fee: u128,
    pub(crate) sequence_base_fee: u128,
    pub(crate) sequence_byte_cost_multiplier: u128,
    pub(crate) init_bridge_account_base_fee: u128,
    pub(crate) bridge_lock_byte_cost_multiplier: u128,
    pub(crate) bridge_sudo_change_fee: u128,
    pub(crate) ics20_withdrawal_base_fee: u128,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Account {
    pub(crate) address: Address,
    pub(crate) balance: u128,
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

    fn genesis_state() -> GenesisState {
        GenesisState {
            accounts: vec![
                Account {
                    address: alice(),
                    balance: 1000000000000000000,
                },
                Account {
                    address: bob(),
                    balance: 1000000000000000000,
                },
                Account {
                    address: charlie(),
                    balance: 1000000000000000000,
                },
            ],
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

    #[test]
    fn snapshots() {
        insta::assert_json_snapshot!(genesis_state());
    }
}
