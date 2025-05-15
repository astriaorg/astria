use std::borrow::Cow;

use astria_core::primitive::v1::asset::IbcPrefixed;

use crate::storage::keys::Asset;

pub(in crate::assets) const NATIVE_ASSET: &str = "assets/native_asset";

/// Example: `assets/ibc/0101....0101`.
///                     |64 hex chars|
pub(in crate::assets) fn asset<'a, TAsset>(asset: &'a TAsset) -> String
where
    &'a TAsset: Into<Cow<'a, IbcPrefixed>>,
{
    format!("assets/{}", Asset::from(asset))
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::asset::Denom;

    use super::*;

    const COMPONENT_PREFIX: &str = "assets/";

    fn test_asset() -> Denom {
        "an/asset/with/a/prefix".parse().unwrap()
    }

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!("native_asset_key", NATIVE_ASSET);
        insta::assert_snapshot!("asset_key_test_asset", asset(&test_asset()));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(NATIVE_ASSET.starts_with(COMPONENT_PREFIX));
        assert!(asset(&test_asset()).starts_with(COMPONENT_PREFIX));
    }
}
