use anyhow::{
    Context as _,
    Result,
};
use astria_core::primitive::v1::{
    asset,
    asset::denom,
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

/// Newtype wrapper to read and write a denomination trace from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct DenominationTrace(String);

fn asset_storage_key<TAsset: Into<asset::IbcPrefixed>>(asset: TAsset) -> String {
    format!("asset/{}", crate::storage_keys::hunks::Asset::from(asset))
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn has_ibc_asset<TAsset>(&self, asset: TAsset) -> Result<bool>
    where
        TAsset: Into<asset::IbcPrefixed> + std::fmt::Display + Send,
    {
        Ok(self
            .get_raw(&asset_storage_key(asset))
            .await
            .context("failed reading raw asset from state")?
            .is_some())
    }

    #[instrument(skip_all)]
    async fn map_ibc_to_trace_prefixed_asset(
        &self,
        asset: asset::IbcPrefixed,
    ) -> Result<Option<denom::TracePrefixed>> {
        let Some(bytes) = self
            .get_raw(&asset_storage_key(asset))
            .await
            .context("failed reading raw asset from state")?
        else {
            return Ok(None);
        };

        let DenominationTrace(denom_str) =
            DenominationTrace::try_from_slice(&bytes).context("invalid asset bytes")?;
        let denom = denom_str
            .parse()
            .context("failed to parse retrieved denom string as a Denom")?;
        Ok(Some(denom))
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_ibc_asset(&mut self, asset: &denom::TracePrefixed) -> Result<()> {
        let bytes = borsh::to_vec(&DenominationTrace(asset.to_string()))
            .context("failed to serialize asset")?;
        self.put_raw(asset_storage_key(asset), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::asset;
    use cnidarium::StateDelta;

    use super::{
        asset_storage_key,
        StateReadExt,
        StateWriteExt as _,
    };

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
    async fn get_ibc_asset_non_existent() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let asset = asset();

        // gets for non existing assets should return none
        assert_eq!(
            state
                .map_ibc_to_trace_prefixed_asset(asset.to_ibc_prefixed())
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
            .put_ibc_asset(&denom.clone().unwrap_trace_prefixed())
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
            .put_ibc_asset(&denom.clone().unwrap_trace_prefixed())
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state
                .map_ibc_to_trace_prefixed_asset(denom.to_ibc_prefixed())
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
            .put_ibc_asset(&denom.clone().unwrap_trace_prefixed())
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state
                .map_ibc_to_trace_prefixed_asset(denom.to_ibc_prefixed())
                .await
                .unwrap()
                .expect("an ibc asset was written and must exist inside the database"),
            denom.clone().unwrap_trace_prefixed(),
            "stored ibc asset was not what was expected"
        );

        // can write another without affecting original
        let denom_1 = asset_1();
        state
            .put_ibc_asset(&denom_1.clone().unwrap_trace_prefixed())
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state
                .map_ibc_to_trace_prefixed_asset(denom_1.to_ibc_prefixed())
                .await
                .unwrap()
                .expect("an additional ibc asset was written and must exist inside the database"),
            denom_1.unwrap_trace_prefixed(),
            "additional ibc asset was not what was expected"
        );
        assert_eq!(
            state
                .map_ibc_to_trace_prefixed_asset(denom.to_ibc_prefixed())
                .await
                .unwrap()
                .expect("an ibc asset was written and must exist inside the database"),
            denom.clone().unwrap_trace_prefixed(),
            "original ibc asset was not what was expected"
        );
    }

    #[test]
    fn storage_keys_are_unchanged() {
        let asset = "an/asset/with/a/prefix"
            .parse::<astria_core::primitive::v1::asset::Denom>()
            .unwrap();
        assert_eq!(
            asset_storage_key(&asset),
            asset_storage_key(asset.to_ibc_prefixed()),
        );
        insta::assert_snapshot!(asset_storage_key(asset));
    }
}
