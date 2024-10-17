use std::borrow::Cow;

use astria_core::primitive::v1::asset;
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

use super::storage::{
    self,
    keys::{
        self,
    },
};
use crate::storage::StoredValue;

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_native_asset(&self) -> Result<Option<asset::TracePrefixed>> {
        let Some(bytes) = self
            .get_raw(keys::NATIVE_ASSET)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read raw native asset from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::TracePrefixedDenom::try_from(value).map(asset::TracePrefixed::from)
            })
            .wrap_err("invalid native asset bytes")
            .map(Option::Some)
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
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    fn asset() -> asset::Denom {
        "asset".parse().unwrap()
    }

    fn asset_0() -> asset::Denom {
        "asset_0".parse().unwrap()
    }
    fn asset_1() -> asset::Denom {
        "asset_1".parse().unwrap()
    }

    #[tokio::test]
    async fn native_asset() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // doesn't exist at first
        assert!(
            state_delta.get_native_asset().await.unwrap().is_none(),
            "no native asset denom should exist at first"
        );

        // can write
        let denom_orig: asset::TracePrefixed = "denom_orig".parse().unwrap();
        state_delta.put_native_asset(denom_orig.clone()).unwrap();
        assert_eq!(
            state_delta.get_native_asset().await.unwrap().expect(
                "a native asset denomination was written and must exist inside the database"
            ),
            denom_orig,
            "stored native asset denomination was not what was expected"
        );

        // can write new value
        let denom_update: asset::TracePrefixed = "denom_update".parse().unwrap();
        state_delta.put_native_asset(denom_update.clone()).unwrap();
        assert_eq!(
            state_delta.get_native_asset().await.unwrap().expect(
                "a native asset denomination update was written and must exist inside the database"
            ),
            denom_update,
            "updated native asset denomination was not what was expected"
        );
    }

    #[tokio::test]
    async fn get_ibc_asset_non_existent() {
        let storage = Storage::new_temp().await;
        let state_delta = storage.new_delta_of_latest_snapshot();

        let asset = asset();

        // gets for non existing assets should return none
        assert_eq!(
            state_delta
                .map_ibc_to_trace_prefixed_asset(&asset.to_ibc_prefixed())
                .await
                .expect("getting non existing asset should not fail"),
            None
        );
    }

    #[tokio::test]
    async fn has_ibc_asset() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let denom = asset();

        // non existing calls are ok for 'has'
        assert!(
            !state_delta
                .has_ibc_asset(&denom)
                .await
                .expect("'has' for non existing ibc assets should be ok"),
            "query for non existing asset should return false"
        );

        state_delta
            .put_ibc_asset(denom.clone().unwrap_trace_prefixed())
            .expect("putting ibc asset should not fail");

        // existing calls are ok for 'has'
        assert!(
            state_delta
                .has_ibc_asset(&denom)
                .await
                .expect("'has' for existing ibc assets should be ok"),
            "query for existing asset should return true"
        );
    }

    #[tokio::test]
    async fn put_ibc_asset_simple() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // can write new
        let denom = asset();
        state_delta
            .put_ibc_asset(denom.clone().unwrap_trace_prefixed())
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state_delta
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
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // can write new
        let denom = asset_0();
        state_delta
            .put_ibc_asset(denom.clone().unwrap_trace_prefixed())
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state_delta
                .map_ibc_to_trace_prefixed_asset(&denom.to_ibc_prefixed())
                .await
                .unwrap()
                .expect("an ibc asset was written and must exist inside the database"),
            denom.clone().unwrap_trace_prefixed(),
            "stored ibc asset was not what was expected"
        );

        // can write another without affecting original
        let denom_1 = asset_1();
        state_delta
            .put_ibc_asset(denom_1.clone().unwrap_trace_prefixed())
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state_delta
                .map_ibc_to_trace_prefixed_asset(&denom_1.to_ibc_prefixed())
                .await
                .unwrap()
                .expect("an additional ibc asset was written and must exist inside the database"),
            denom_1.unwrap_trace_prefixed(),
            "additional ibc asset was not what was expected"
        );
        assert_eq!(
            state_delta
                .map_ibc_to_trace_prefixed_asset(&denom.to_ibc_prefixed())
                .await
                .unwrap()
                .expect("an ibc asset was written and must exist inside the database"),
            denom.clone().unwrap_trace_prefixed(),
            "original ibc asset was not what was expected"
        );
    }
}
