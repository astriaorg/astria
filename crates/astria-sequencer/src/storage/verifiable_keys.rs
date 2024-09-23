use std::{
    fmt::{
        self,
        Display,
        Formatter,
    },
    str::FromStr,
};

use astria_core::primitive::v1::asset::IbcPrefixed;
use astria_eyre::eyre::{
    ContextCompat as _,
    Result,
    WrapErr as _,
};

use crate::accounts::AddressBytes;

pub(crate) mod accounts {
    use super::*;

    pub(crate) const TRANSFER_BASE_FEE_KEY: &str = "transferfee";
    const ACCOUNTS_PREFIX: &str = "accounts/";
    const BALANCE_PREFIX: &str = "balance/";
    const NONCE: &str = "nonce";

    /// Example: `accounts/0101....0101/balance/`.
    ///                   |40 hex chars|
    pub(crate) fn balance_prefix<TAddress: AddressBytes>(address: &TAddress) -> String {
        format!(
            "{}/{BALANCE_PREFIX}",
            AddressPrefixer::new(ACCOUNTS_PREFIX, address)
        )
    }

    /// Example: `accounts/0101....0101/balance/0202....0202`.
    ///                   |40 hex chars|       |64 hex chars|
    pub(crate) fn balance_key<TAddress: AddressBytes, TAsset: Into<IbcPrefixed>>(
        address: TAddress,
        asset: TAsset,
    ) -> String {
        format!(
            "{}/{BALANCE_PREFIX}{}",
            AddressPrefixer::new(ACCOUNTS_PREFIX, &address),
            Asset::from(asset)
        )
    }

    /// Example: `accounts/0101....0101/nonce`.
    ///                   |40 hex chars|
    pub(crate) fn nonce_key<TAddress: AddressBytes>(address: &TAddress) -> String {
        format!("{}/{NONCE}", AddressPrefixer::new(ACCOUNTS_PREFIX, address))
    }

    pub(crate) fn extract_asset_from_key(key: &str) -> Result<IbcPrefixed> {
        Ok(key
            .strip_prefix(ACCOUNTS_PREFIX)
            .and_then(|s| s.split_once(BALANCE_PREFIX).map(|(_, asset)| asset))
            .wrap_err("failed to strip prefix from account balance key")?
            .parse::<Asset>()
            .wrap_err("failed to parse storage key suffix as address hunk")?
            .0)
    }

    #[cfg(test)]
    mod tests {
        use astria_core::primitive::v1::{
            asset::Denom,
            Address,
        };

        use super::*;

        #[test]
        fn keys_should_not_change() {
            let address: Address = "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
                .parse()
                .unwrap();
            let asset = "an/asset/with/a/prefix".parse::<Denom>().unwrap();
            assert_eq!(
                balance_key(address, &asset),
                balance_key(address, asset.to_ibc_prefixed())
            );
            insta::assert_snapshot!(
                balance_prefix(&address),
                @"accounts/1c0c490f1b5528d8173c5de46d131160e4b2c0c3/balance/"
            );
            insta::assert_snapshot!(
                balance_key(address, asset),
                @"accounts/1c0c490f1b5528d8173c5de46d131160e4b2c0c3/balance/be429a02d00837245167a26\
                16674a979a2ac6f9806468b48a975b156ad711320"
            );
            insta::assert_snapshot!(
                nonce_key(&address),
                @"accounts/1c0c490f1b5528d8173c5de46d131160e4b2c0c3/nonce"
            );
        }

        #[test]
        fn balance_prefix_should_be_prefix_of_balance_key() {
            let address: Address = "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
                .parse()
                .unwrap();
            let asset = "an/asset/with/a/prefix".parse::<Denom>().unwrap();
            let prefix = balance_prefix(&address);
            let key = balance_key(address, asset);
            assert!(key.strip_prefix(&prefix).is_some());
        }

        #[test]
        fn should_extract_asset_from_key() {
            let asset = IbcPrefixed::new([2; 32]);
            let key = balance_key([1; 20], asset);
            let recovered_asset = extract_asset_from_key(&key).unwrap();
            assert_eq!(asset, recovered_asset);
        }
    }
}

pub(crate) mod address {
    pub(crate) const BASE_PREFIX_KEY: &str = "prefixes/base";
    pub(crate) const IBC_COMPAT_PREFIX_KEY: &str = "prefixes/ibc-compat";
}

pub(crate) mod app {
    pub(crate) const CHAIN_ID_KEY: &str = "chain_id";
    pub(crate) const REVISION_NUMBER_KEY: &str = "revision_number";
    pub(crate) const BLOCK_HEIGHT_KEY: &str = "block_height";
    pub(crate) const BLOCK_TIMESTAMP_KEY: &str = "block_timestamp";
}

pub(crate) mod assets {
    use astria_core::primitive::v1::asset::IbcPrefixed;

    use crate::storage::verifiable_keys::Asset;

    pub(crate) const ASSET_PREFIX: &str = "asset/";
    pub(crate) const NATIVE_ASSET_KEY: &str = "nativeasset";

    /// Example: `asset/0101....0101`.
    ///                |64 hex chars|
    pub(crate) fn asset_key<TAsset: Into<IbcPrefixed>>(asset: TAsset) -> String {
        format!("{ASSET_PREFIX}{}", Asset::from(asset))
    }

    #[cfg(test)]
    mod tests {
        use astria_core::primitive::v1::asset::Denom;

        use super::*;

        #[test]
        fn keys_should_not_change() {
            let asset = "an/asset/with/a/prefix".parse::<Denom>().unwrap();
            insta::assert_snapshot!(
                asset_key(asset),
                @"asset/be429a02d00837245167a2616674a979a2ac6f9806468b48a975b156ad711320"
            );
        }
    }
}

pub(crate) mod authority {
    pub(crate) const SUDO_STORAGE_KEY: &str = "sudo";
    pub(crate) const VALIDATOR_SET_STORAGE_KEY: &str = "valset";
}

pub(crate) mod bridge {
    use crate::{
        accounts::AddressBytes,
        storage::verifiable_keys::AddressPrefixer,
    };

    pub(crate) const INIT_BRIDGE_ACCOUNT_BASE_FEE_KEY: &str = "initbridgeaccfee";
    pub(crate) const BRIDGE_LOCK_BYTE_COST_MULTIPLIER_KEY: &str = "bridgelockmultiplier";
    pub(crate) const BRIDGE_SUDO_CHANGE_FEE_KEY: &str = "bridgesudofee";

    pub(in crate::storage) const BRIDGE_ACCOUNT_PREFIX: &str = "bridgeacc/";
    const BRIDGE_ACCOUNT_SUDO_PREFIX: &str = "bsudo/";
    const BRIDGE_ACCOUNT_WITHDRAWER_PREFIX: &str = "bwithdrawer/";

    /// Example: `bridgeacc/0101....0101/rollupid`.
    ///                    |40 hex chars|
    pub(crate) fn rollup_id_key<T: AddressBytes>(address: &T) -> String {
        format!(
            "{}/rollupid",
            AddressPrefixer::new(BRIDGE_ACCOUNT_PREFIX, address)
        )
    }

    /// Example: `bridgeacc/0101....0101/assetid`.
    ///                    |40 hex chars|
    pub(crate) fn asset_id_key<T: AddressBytes>(address: &T) -> String {
        format!(
            "{}/assetid",
            AddressPrefixer::new(BRIDGE_ACCOUNT_PREFIX, address)
        )
    }

    /// Example: `bsudo/0101....0101`.
    ///                |40 hex chars|
    pub(crate) fn bridge_account_sudo_address_key<T: AddressBytes>(address: &T) -> String {
        AddressPrefixer::new(BRIDGE_ACCOUNT_SUDO_PREFIX, address).to_string()
    }

    /// Example: `bwithdrawer/0101....0101`.
    ///                      |40 hex chars|
    pub(crate) fn bridge_account_withdrawer_address_key<T: AddressBytes>(address: &T) -> String {
        AddressPrefixer::new(BRIDGE_ACCOUNT_WITHDRAWER_PREFIX, address).to_string()
    }

    /// Example: `bridgeacc/0101....0101/withdrawalevent/<event id>`.
    ///                    |40 hex chars|               |UTF-8 chars|
    pub(crate) fn bridge_account_withdrawal_event_key<T: AddressBytes>(
        address: &T,
        withdrawal_event_id: &str,
    ) -> String {
        format!(
            "{}/withdrawalevent/{}",
            AddressPrefixer::new(BRIDGE_ACCOUNT_PREFIX, address),
            withdrawal_event_id
        )
    }

    #[cfg(test)]
    mod tests {
        use astria_core::primitive::v1::Address;

        use super::*;

        #[test]
        fn keys_should_not_change() {
            let address: Address = "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
                .parse()
                .unwrap();

            insta::assert_snapshot!(
                rollup_id_key(&address),
                @"bridgeacc/1c0c490f1b5528d8173c5de46d131160e4b2c0c3/rollupid"
            );
            insta::assert_snapshot!(
                asset_id_key(&address),
                @"bridgeacc/1c0c490f1b5528d8173c5de46d131160e4b2c0c3/assetid"
            );
            insta::assert_snapshot!(
                bridge_account_sudo_address_key(&address),
                @"bsudo/1c0c490f1b5528d8173c5de46d131160e4b2c0c3"
            );
            insta::assert_snapshot!(
                bridge_account_withdrawer_address_key(&address),
                @"bwithdrawer/1c0c490f1b5528d8173c5de46d131160e4b2c0c3"
            );
            insta::assert_snapshot!(
                bridge_account_withdrawal_event_key(&address, "the-event"),
                @"bridgeacc/1c0c490f1b5528d8173c5de46d131160e4b2c0c3/withdrawalevent/the-event"
            );
        }
    }
}

pub(crate) mod ibc {
    use astria_core::primitive::v1::asset::IbcPrefixed;
    use ibc_types::core::channel::ChannelId;

    use crate::{
        accounts::AddressBytes,
        storage::verifiable_keys::{
            AddressPrefixer,
            Asset,
        },
    };

    pub(crate) const IBC_SUDO_KEY: &str = "ibcsudo";
    pub(crate) const ICS20_WITHDRAWAL_BASE_FEE_KEY: &str = "ics20withdrawalfee";

    const IBC_RELAYER_PREFIX: &str = "ibc-relayer/";

    /// Example: `ibc-data/channel-xxx/balance/0101....0101`.
    ///                           |int|       |64 hex chars|
    pub(crate) fn channel_balance_key<TAsset: Into<IbcPrefixed>>(
        channel: &ChannelId,
        asset: TAsset,
    ) -> String {
        format!("ibc-data/{channel}/balance/{}", Asset::from(asset),)
    }

    /// Example: `ibc-relayer/0101....0101`.
    ///                      |40 hex chars|
    pub(crate) fn ibc_relayer_key<T: AddressBytes>(address: &T) -> String {
        AddressPrefixer::new(IBC_RELAYER_PREFIX, address).to_string()
    }

    #[cfg(test)]
    mod tests {
        use astria_core::primitive::v1::Address;

        use super::*;

        #[test]
        fn keys_should_not_change() {
            let channel = ChannelId::new(5);
            let asset = "an/asset/with/a/prefix"
                .parse::<astria_core::primitive::v1::asset::Denom>()
                .unwrap();
            assert_eq!(
                channel_balance_key(&channel, &asset),
                channel_balance_key(&channel, asset.to_ibc_prefixed()),
            );
            insta::assert_snapshot!(
                channel_balance_key(&channel, &asset),
                @"ibc-data/channel-5/balance/be429a02d00837245167a2616674a979a2ac6f9806468b48a975b1\
                56ad711320"
            );

            let address: Address = "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
                .parse()
                .unwrap();
            insta::assert_snapshot!(
                ibc_relayer_key(&address),
                @"ibc-relayer/1c0c490f1b5528d8173c5de46d131160e4b2c0c3"
            );
        }
    }
}

pub(crate) mod sequence {
    pub(crate) const SEQUENCE_ACTION_BASE_FEE_KEY: &str = "seqbasefee";
    pub(crate) const SEQUENCE_ACTION_BYTE_COST_MULTIPLIER_KEY: &str = "seqmultiplier";
}

/// Helper struct whose `Display` impl outputs the prefix followed by the hex-encoded address.
pub(super) struct AddressPrefixer<'a, T> {
    prefix: &'static str,
    address: &'a T,
}

impl<'a, T> AddressPrefixer<'a, T> {
    pub(in crate::storage) fn new(prefix: &'static str, address: &'a T) -> Self {
        Self {
            prefix,
            address,
        }
    }
}

impl<'a, T: AddressBytes> Display for AddressPrefixer<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            self.prefix,
            hex::encode(self.address.address_bytes())
        )
    }
}

/// Helper struct whose `Display` impl outputs the hex-encoded ibc-prefixed address, and that can be
/// parsed from such a hex-encoded form.
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(super) struct Asset(IbcPrefixed);

impl Asset {
    pub(super) fn get(self) -> IbcPrefixed {
        self.0
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0.get()))
    }
}

impl<T: Into<IbcPrefixed>> From<T> for Asset {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to parse input as asset key")]
pub(super) struct ParseAssetKeyError {
    #[from]
    source: hex::FromHexError,
}

impl FromStr for Asset {
    type Err = ParseAssetKeyError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        use hex::FromHex as _;
        let bytes = <[u8; 32]>::from_hex(s)?;
        Ok(Self(IbcPrefixed::new(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::Asset;

    #[test]
    fn asset_key_to_string_parse_roundtrip() {
        let asset = "an/asset/with/a/prefix"
            .parse::<astria_core::primitive::v1::asset::Denom>()
            .unwrap();
        let expected = Asset::from(asset);
        let actual = expected.to_string().parse::<Asset>().unwrap();
        assert_eq!(expected, actual);
    }
}
