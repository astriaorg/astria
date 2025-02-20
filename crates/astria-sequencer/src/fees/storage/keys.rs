use std::borrow::Cow;

use astria_core::primitive::v1::asset::IbcPrefixed;
use astria_eyre::eyre::{
    self,
    eyre,
    Context as _,
};

use crate::{
    fees::FeeHandler,
    storage::keys::Asset,
};

pub(in crate::fees) const BLOCK: &str = "fees/block"; // NOTE: `BLOCK` is only used in the ephemeral store.
pub(in crate::fees) const ALLOWED_ASSET_PREFIX: &str = "fees/allowed_asset/";
pub(in crate::fees) fn name<F: FeeHandler + ?Sized>() -> String {
    format!("fees/{}", F::snake_case_name())
}

pub(in crate::fees) fn allowed_asset<'a, TAsset>(asset: &'a TAsset) -> String
where
    &'a TAsset: Into<Cow<'a, IbcPrefixed>>,
{
    format!("{ALLOWED_ASSET_PREFIX}{}", Asset::from(asset))
}

pub(in crate::fees) fn extract_asset_from_allowed_asset_key(
    key: &str,
) -> eyre::Result<IbcPrefixed> {
    extract_asset_from_key(key, ALLOWED_ASSET_PREFIX)
        .wrap_err("failed to extract asset from fee asset key")
}

fn extract_asset_from_key(key: &str, prefix: &str) -> eyre::Result<IbcPrefixed> {
    let suffix = key
        .strip_prefix(prefix)
        .ok_or_else(|| eyre!("key `{key}` did not have prefix `{prefix}`"))?;
    suffix.parse().wrap_err_with(|| {
        format!("failed to parse suffix `{suffix}` of key `{key}` as an ibc-prefixed asset",)
    })
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::asset::Denom,
        protocol::transaction::v1::action::{
            BridgeLock,
            BridgeSudoChange,
            BridgeTransfer,
            BridgeUnlock,
            FeeAssetChange,
            FeeChange,
            IbcRelayerChange,
            IbcSudoChange,
            Ics20Withdrawal,
            InitBridgeAccount,
            RollupDataSubmission,
            SudoAddressChange,
            Transfer,
            ValidatorUpdate,
        },
    };
    use insta::assert_snapshot;
    use penumbra_ibc::IbcRelay;

    use super::*;

    const COMPONENT_PREFIX: &str = "fees/";

    fn test_asset() -> Denom {
        "an/asset/with/a/prefix".parse().unwrap()
    }

    #[test]
    fn keys_should_not_change() {
        // NOTE: `BLOCK` is only used in the ephemeral store, so isn't included here.

        fn check<F: FeeHandler>() {
            assert_snapshot!(format!("{}_fees_key", F::snake_case_name()), name::<F>());
        }

        check::<BridgeLock>();
        check::<BridgeSudoChange>();
        check::<BridgeUnlock>();
        check::<FeeAssetChange>();
        check::<FeeChange>();
        check::<IbcRelay>();
        check::<IbcRelayerChange>();
        check::<IbcSudoChange>();
        check::<Ics20Withdrawal>();
        check::<InitBridgeAccount>();
        check::<RollupDataSubmission>();
        check::<SudoAddressChange>();
        check::<Transfer>();
        check::<ValidatorUpdate>();
        check::<BridgeTransfer>();
        assert_snapshot!("allowed_asset_prefix", ALLOWED_ASSET_PREFIX);
        assert_snapshot!("allowed_asset_key", allowed_asset(&test_asset()));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(name::<Transfer>().starts_with(COMPONENT_PREFIX));
        assert!(name::<RollupDataSubmission>().starts_with(COMPONENT_PREFIX));
        assert!(name::<Ics20Withdrawal>().starts_with(COMPONENT_PREFIX));
        assert!(name::<InitBridgeAccount>().starts_with(COMPONENT_PREFIX));
        assert!(name::<BridgeLock>().starts_with(COMPONENT_PREFIX));
        assert!(name::<BridgeUnlock>().starts_with(COMPONENT_PREFIX));
        assert!(name::<BridgeSudoChange>().starts_with(COMPONENT_PREFIX));
        assert!(name::<IbcRelay>().starts_with(COMPONENT_PREFIX));
        assert!(name::<ValidatorUpdate>().starts_with(COMPONENT_PREFIX));
        assert!(name::<FeeAssetChange>().starts_with(COMPONENT_PREFIX));
        assert!(name::<FeeChange>().starts_with(COMPONENT_PREFIX));
        assert!(name::<IbcRelayerChange>().starts_with(COMPONENT_PREFIX));
        assert!(name::<SudoAddressChange>().starts_with(COMPONENT_PREFIX));
        assert!(name::<IbcSudoChange>().starts_with(COMPONENT_PREFIX));
        assert!(name::<BridgeTransfer>().starts_with(COMPONENT_PREFIX));
        assert!(ALLOWED_ASSET_PREFIX.starts_with(COMPONENT_PREFIX));
        assert!(allowed_asset(&test_asset()).starts_with(COMPONENT_PREFIX));
    }

    #[test]
    fn prefixes_should_be_prefixes_of_relevant_keys() {
        assert!(allowed_asset(&test_asset()).starts_with(ALLOWED_ASSET_PREFIX));
    }

    #[test]
    fn should_extract_asset_from_key() {
        let asset = IbcPrefixed::new([1; 32]);

        let key = allowed_asset(&asset);
        let recovered_asset = extract_asset_from_allowed_asset_key(&key).unwrap();
        assert_eq!(asset, recovered_asset);
    }
}
