use std::borrow::Cow;

use astria_core::primitive::v1::asset::IbcPrefixed;
use ibc_types::core::channel::ChannelId;

use crate::{
    accounts::AddressBytes,
    storage::keys::{
        AccountPrefixer,
        Asset,
    },
};

pub(in crate::ibc) const IBC_SUDO: &str = "ibc/sudo";
pub(in crate::ibc) const CONTEXT_EPHEMERAL: &str = "ibc/context";
const IBC_RELAYER_PREFIX: &str = "ibc/relayer/";

/// Example: `ibc/channel-xxx/balance/ibc/0101....0101`.
///                      |int|           |64 hex chars|
pub(in crate::ibc) fn channel_balance<'a, TAsset>(channel: &ChannelId, asset: &'a TAsset) -> String
where
    &'a TAsset: Into<Cow<'a, IbcPrefixed>>,
{
    format!("ibc/{channel}/balance/{}", Asset::from(asset))
}

/// Example: `ibc/relayer/gGhH....zZ4=`.
///                      |base64 chars|
pub(in crate::ibc) fn ibc_relayer<T: AddressBytes>(address: &T) -> String {
    AccountPrefixer::new(IBC_RELAYER_PREFIX, address).to_string()
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
        insta::assert_snapshot!("ibc_sudo_key", IBC_SUDO);
        insta::assert_snapshot!("ibc_context_key", CONTEXT_EPHEMERAL);
        insta::assert_snapshot!(
            "channel_balance_key",
            channel_balance(&channel_id(), &asset())
        );
        insta::assert_snapshot!("ibc_relayer_key", ibc_relayer(&address()));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(IBC_SUDO.starts_with(COMPONENT_PREFIX));
        assert!(CONTEXT_EPHEMERAL.starts_with(COMPONENT_PREFIX));
        assert!(channel_balance(&channel_id(), &asset()).starts_with(COMPONENT_PREFIX));
        assert!(ibc_relayer(&address()).starts_with(COMPONENT_PREFIX));
    }
}
