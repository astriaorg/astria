use std::borrow::Cow;

use astria_core::primitive::v1::asset::IbcPrefixed;
use ibc_types::core::channel::ChannelId;

use crate::{
    accounts::AddressBytes,
    storage::keys::{
        AddressPrefixer,
        Asset,
    },
};

pub(in crate::ibc) const IBC_SUDO_KEY: &str = "ibc/sudo";
pub(in crate::ibc) const ICS20_WITHDRAWAL_BASE_FEE_KEY: &str = "ibc/ics20_withdrawal_base_fee";
const IBC_RELAYER_PREFIX: &str = "ibc/relayer/";

/// Example: `ibc/channel-xxx/balance/0101....0101`.
///                      |int|       |64 hex chars|
pub(in crate::ibc) fn channel_balance_key<'a, TAsset>(
    channel: &ChannelId,
    asset: &'a TAsset,
) -> String
where
    &'a TAsset: Into<Cow<'a, IbcPrefixed>>,
{
    format!("ibc/{channel}/balance/{}", Asset::from(asset))
}

/// Example: `ibc/relayer/0101....0101`.
///                      |40 hex chars|
pub(in crate::ibc) fn ibc_relayer_key<T: AddressBytes>(address: &T) -> String {
    AddressPrefixer::new(IBC_RELAYER_PREFIX, address).to_string()
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::{
        asset::Denom,
        Address,
    };

    use super::*;

    const COMPONENT_PREFIX: &str = "ibc/";

    fn channel_id() -> ChannelId {
        ChannelId::new(5)
    }

    fn address() -> Address {
        "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
            .parse()
            .unwrap()
    }

    fn asset() -> Denom {
        "an/asset/with/a/prefix".parse().unwrap()
    }

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(IBC_SUDO_KEY);
        insta::assert_snapshot!(ICS20_WITHDRAWAL_BASE_FEE_KEY);
        insta::assert_snapshot!(channel_balance_key(&channel_id(), &asset()));
        insta::assert_snapshot!(ibc_relayer_key(&address()));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(IBC_SUDO_KEY.starts_with(COMPONENT_PREFIX));
        assert!(ICS20_WITHDRAWAL_BASE_FEE_KEY.starts_with(COMPONENT_PREFIX));
        assert!(channel_balance_key(&channel_id(), &asset()).starts_with(COMPONENT_PREFIX));
        assert!(ibc_relayer_key(&address()).starts_with(COMPONENT_PREFIX));
    }
}
