use std::borrow::Cow;

use astria_core::primitive::v1::asset;
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        bail,
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use futures::StreamExt as _;
use tracing::instrument;

use super::storage::{
    self,
    keys::{
        self,
        extract_asset_from_fee_asset_key,
    },
};
use crate::storage::StoredValue;

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_native_asset(&self) -> Result<asset::TracePrefixed> {
        let Some(bytes) = self
            .get_raw(keys::NATIVE_ASSET)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read raw native asset from state")?
        else {
            bail!("native asset denom not found in state");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::TracePrefixedDenom::try_from(value).map(asset::TracePrefixed::from)
            })
            .wrap_err("invalid native asset bytes")
    }

    #[instrument(skip_all)]
    async fn has_ibc_asset<'a, TAsset>(&self, asset: &'a TAsset) -> Result<bool>
    where
        TAsset: Sync,
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        Ok(self
            .get_raw(&keys::asset(asset))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw asset from state")?
            .is_some())
    }

    #[instrument(skip_all, fields(%asset), err)]
    async fn map_ibc_to_trace_prefixed_asset(
        &self,
        asset: &asset::IbcPrefixed,
    ) -> Result<Option<asset::TracePrefixed>> {
        let Some(bytes) = self
            .get_raw(&keys::asset(asset))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw asset from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::TracePrefixedDenom::try_from(value)
                    .map(|stored_denom| Some(asset::TracePrefixed::from(stored_denom)))
            })
            .wrap_err("invalid ibc asset bytes")
    }

    #[instrument(skip_all)]
    async fn is_allowed_fee_asset<'a, TAsset>(&self, asset: &'a TAsset) -> Result<bool>
    where
        TAsset: Sync,
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        Ok(self
            .nonverifiable_get_raw(keys::fee_asset(asset).as_bytes())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read raw fee asset from state")?
            .is_some())
    }

    #[instrument(skip_all)]
    async fn get_allowed_fee_assets(&self) -> Result<Vec<asset::IbcPrefixed>> {
        let mut assets = Vec::new();

        let mut stream =
            std::pin::pin!(self.nonverifiable_prefix_raw(keys::FEE_ASSET_PREFIX.as_bytes()));
        while let Some(Ok((key, _))) = stream.next().await {
            let asset =
                extract_asset_from_fee_asset_key(&key).wrap_err("failed to extract asset")?;
            assets.push(asset);
        }

        Ok(assets)
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_native_asset(&mut self, asset: asset::TracePrefixed) -> Result<()> {
        let bytes = StoredValue::from(storage::TracePrefixedDenom::from(&asset))
            .serialize()
            .context("failed to serialize native asset")?;
        self.put_raw(keys::NATIVE_ASSET.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_ibc_asset(&mut self, asset: asset::TracePrefixed) -> Result<()> {
        let key = keys::asset(&asset);
        let bytes = StoredValue::from(storage::TracePrefixedDenom::from(&asset))
            .serialize()
            .wrap_err("failed to serialize ibc asset")?;
        self.put_raw(key, bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn delete_allowed_fee_asset<'a, TAsset>(&mut self, asset: &'a TAsset)
    where
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        self.nonverifiable_delete(keys::fee_asset(asset).into_bytes());
    }

    #[instrument(skip_all)]
    fn put_allowed_fee_asset<'a, TAsset>(&mut self, asset: &'a TAsset) -> Result<()>
    where
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        let bytes = StoredValue::Unit
            .serialize()
            .context("failed to serialize unit for allowed fee asset")?;
        self.nonverifiable_put_raw(keys::fee_asset(asset).into_bytes(), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use cnidarium::StateDelta;

    use super::*;

    fn asset() -> asset::Denom {
        "asset".parse().unwrap()
    }

    fn asset_0() -> asset::Denom {
        "asset_0".parse().unwrap()
    }
    fn asset_1() -> asset::Denom {
        "asset_1".parse().unwrap()
    }
    fn asset_2() -> asset::Denom {
        "asset_2".parse().unwrap()
    }

    #[tokio::test]
    async fn native_asset() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        let _ = state
            .get_native_asset()
            .await
            .expect_err("no native asset denom should exist at first");

        // can write
        let denom_orig: asset::TracePrefixed = "denom_orig".parse().unwrap();
        state.put_native_asset(denom_orig.clone()).unwrap();
        assert_eq!(
            state.get_native_asset().await.expect(
                "a native asset denomination was written and must exist inside the database"
            ),
            denom_orig,
            "stored native asset denomination was not what was expected"
        );

        // can write new value
        let denom_update: asset::TracePrefixed = "denom_update".parse().unwrap();
        state.put_native_asset(denom_update.clone()).unwrap();
        assert_eq!(
            state.get_native_asset().await.expect(
                "a native asset denomination update was written and must exist inside the database"
            ),
            denom_update,
            "updated native asset denomination was not what was expected"
        );
    }

    #[tokio::test]
    async fn get_ibc_asset_non_existent() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let asset = asset();

        // gets for non existing assets should return none
        assert_eq!(
            state
                .map_ibc_to_trace_prefixed_asset(&asset.to_ibc_prefixed())
                .await
                .expect("getting non existing asset should not fail"),
            None
        );
    }

    #[tokio::test]
    async fn has_ibc_asset() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let denom = asset();

        // non existing calls are ok for 'has'
        assert!(
            !state
                .has_ibc_asset(&denom)
                .await
                .expect("'has' for non existing ibc assets should be ok"),
            "query for non existing asset should return false"
        );

        state
            .put_ibc_asset(denom.clone().unwrap_trace_prefixed())
            .expect("putting ibc asset should not fail");

        // existing calls are ok for 'has'
        assert!(
            state
                .has_ibc_asset(&denom)
                .await
                .expect("'has' for existing ibc assets should be ok"),
            "query for existing asset should return true"
        );
    }

    #[tokio::test]
    async fn put_ibc_asset_simple() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write new
        let denom = asset();
        state
            .put_ibc_asset(denom.clone().unwrap_trace_prefixed())
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state
                .map_ibc_to_trace_prefixed_asset(&denom.to_ibc_prefixed())
                .await
                .unwrap()
                .expect("an ibc asset was written and must exist inside the database"),
            denom.unwrap_trace_prefixed(),
            "stored ibc asset was not what was expected"
        );
    }

    #[tokio::test]
    async fn put_ibc_asset_complex() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write new
        let denom = asset_0();
        state
            .put_ibc_asset(denom.clone().unwrap_trace_prefixed())
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state
                .map_ibc_to_trace_prefixed_asset(&denom.to_ibc_prefixed())
                .await
                .unwrap()
                .expect("an ibc asset was written and must exist inside the database"),
            denom.clone().unwrap_trace_prefixed(),
            "stored ibc asset was not what was expected"
        );

        // can write another without affecting original
        let denom_1 = asset_1();
        state
            .put_ibc_asset(denom_1.clone().unwrap_trace_prefixed())
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state
                .map_ibc_to_trace_prefixed_asset(&denom_1.to_ibc_prefixed())
                .await
                .unwrap()
                .expect("an additional ibc asset was written and must exist inside the database"),
            denom_1.unwrap_trace_prefixed(),
            "additional ibc asset was not what was expected"
        );
        assert_eq!(
            state
                .map_ibc_to_trace_prefixed_asset(&denom.to_ibc_prefixed())
                .await
                .unwrap()
                .expect("an ibc asset was written and must exist inside the database"),
            denom.clone().unwrap_trace_prefixed(),
            "original ibc asset was not what was expected"
        );
    }

    #[tokio::test]
    async fn is_allowed_fee_asset() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // non-existent fees assets return false
        let asset = asset_0();
        assert!(
            !state
                .is_allowed_fee_asset(&asset)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to return false"
        );

        // existent fee assets return true
        state.put_allowed_fee_asset(&asset).unwrap();
        assert!(
            state
                .is_allowed_fee_asset(&asset)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
    }

    #[tokio::test]
    async fn can_delete_allowed_fee_assets_simple() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // setup fee asset
        let asset = asset_0();
        state.put_allowed_fee_asset(&asset).unwrap();
        assert!(
            state
                .is_allowed_fee_asset(&asset)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );

        // see can get fee asset
        let assets = state.get_allowed_fee_assets().await.unwrap();
        assert_eq!(
            assets,
            vec![asset.to_ibc_prefixed()],
            "expected returned allowed fee assets to match what was written in"
        );

        // can delete
        state.delete_allowed_fee_asset(&asset);

        // see is deleted
        let assets = state.get_allowed_fee_assets().await.unwrap();
        assert!(assets.is_empty(), "fee assets should be empty post delete");
    }

    #[tokio::test]
    async fn can_delete_allowed_fee_assets_complex() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // setup fee assets
        let asset_first = asset_0();
        state.put_allowed_fee_asset(&asset_first).unwrap();
        assert!(
            state
                .is_allowed_fee_asset(&asset_first)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
        let asset_second = asset_1();
        state.put_allowed_fee_asset(&asset_second).unwrap();
        assert!(
            state
                .is_allowed_fee_asset(&asset_second)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
        let asset_third = asset_2();
        state.put_allowed_fee_asset(&asset_third).unwrap();
        assert!(
            state
                .is_allowed_fee_asset(&asset_third)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );

        // can delete
        state.delete_allowed_fee_asset(&asset_second);

        // see is deleted
        let assets = HashSet::<_>::from_iter(state.get_allowed_fee_assets().await.unwrap());
        assert_eq!(
            assets,
            HashSet::from_iter(vec![
                asset_first.to_ibc_prefixed(),
                asset_third.to_ibc_prefixed()
            ]),
            "delete for allowed fee asset did not behave as expected"
        );
    }
}
