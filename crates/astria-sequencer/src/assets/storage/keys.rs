use std::borrow::Cow;

use astria_core::primitive::v1::asset::IbcPrefixed;
use astria_eyre::eyre::{
    bail,
    ContextCompat,
    Result,
    WrapErr,
};

use crate::storage::keys::Asset;

pub(in crate::assets) const NATIVE_ASSET: &str = "assets/native_asset";
pub(in crate::assets) const BLOCK_FEES_PREFIX: &[u8] = b"assets/block_fees/";
pub(in crate::assets) const FEE_ASSET_PREFIX: &[u8] = b"assets/fee_asset/";

/// Example: `assets/0101....0101`.
///                 |64 hex chars|
pub(in crate::assets) fn asset<'a, TAsset>(asset: &'a TAsset) -> String
where
    &'a TAsset: Into<Cow<'a, IbcPrefixed>>,
{
    format!("assets/{}", Asset::from(asset))
}

pub(in crate::assets) fn fee_asset<'a, TAsset>(asset: &'a TAsset) -> Vec<u8>
where
    &'a TAsset: Into<Cow<'a, IbcPrefixed>>,
{
    [FEE_ASSET_PREFIX, Asset::from(asset).as_bytes()].concat()
}

pub(in crate::assets) fn block_fees<'a, TAsset>(asset: &'a TAsset) -> Vec<u8>
where
    &'a TAsset: Into<Cow<'a, IbcPrefixed>>,
{
    [BLOCK_FEES_PREFIX, Asset::from(asset).as_bytes()].concat()
}

pub(in crate::assets) fn extract_asset_from_fee_asset_key(key: &[u8]) -> Result<IbcPrefixed> {
    extract_asset_from_key(key, FEE_ASSET_PREFIX)
        .wrap_err("failed to extract asset from fee asset key")
}

pub(in crate::assets) fn extract_asset_from_block_fees_key(key: &[u8]) -> Result<IbcPrefixed> {
    extract_asset_from_key(key, BLOCK_FEES_PREFIX)
        .wrap_err("failed to extract asset from fee asset key")
}

fn extract_asset_from_key(key: &[u8], prefix: &[u8]) -> Result<IbcPrefixed> {
    let suffix = key.strip_prefix(prefix).wrap_err_with(|| {
        format!(
            "key `{}` did not have prefix `{}`",
            telemetry::display::hex(key),
            telemetry::display::hex(prefix)
        )
    })?;
    if suffix.len() != IbcPrefixed::ENCODED_HASH_LEN {
        bail!(
            "suffix `{}` of key `{}` is not {} bytes",
            telemetry::display::hex(suffix),
            telemetry::display::hex(key),
            IbcPrefixed::ENCODED_HASH_LEN
        );
    }
    let mut buffer = [0; IbcPrefixed::ENCODED_HASH_LEN];
    buffer.copy_from_slice(suffix);
    Ok(IbcPrefixed::new(buffer))
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::asset::Denom;
    use telemetry::display::base64;

    use super::*;

    const COMPONENT_PREFIX: &str = "assets/";

    fn test_asset() -> Denom {
        "an/asset/with/a/prefix".parse().unwrap()
    }

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(NATIVE_ASSET);
        insta::assert_snapshot!(asset(&test_asset()));
        insta::assert_snapshot!(base64(&fee_asset(&test_asset())));
        insta::assert_snapshot!(base64(&block_fees(&test_asset())));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(NATIVE_ASSET.starts_with(COMPONENT_PREFIX));
        assert!(asset(&test_asset()).starts_with(COMPONENT_PREFIX));
        assert!(fee_asset(&test_asset()).starts_with(COMPONENT_PREFIX.as_bytes()));
        assert!(block_fees(&test_asset()).starts_with(COMPONENT_PREFIX.as_bytes()));
    }

    #[test]
    fn prefixes_should_be_prefixes_of_relevant_keys() {
        assert!(fee_asset(&test_asset()).starts_with(FEE_ASSET_PREFIX));
        assert!(block_fees(&test_asset()).starts_with(BLOCK_FEES_PREFIX));
    }

    #[test]
    fn should_extract_asset_from_key() {
        let asset = IbcPrefixed::new([1; 32]);

        let key = fee_asset(&asset);
        let recovered_asset = extract_asset_from_fee_asset_key(&key).unwrap();
        assert_eq!(asset, recovered_asset);

        let key = block_fees(&asset);
        let recovered_asset = extract_asset_from_block_fees_key(&key).unwrap();
        assert_eq!(asset, recovered_asset);
    }
}
