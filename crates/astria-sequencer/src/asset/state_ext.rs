use anyhow::{
    bail,
    Context as _,
    Result,
};
use astria_core::primitive::v1::{
    asset,
    asset::Denom,
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
use hex::ToHex as _;
use tracing::instrument;

/// Newtype wrapper to read and write a denomination trace from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct DenominationTrace(String);

fn asset_storage_key(asset: asset::Id) -> String {
    format!("asset/{}", asset.encode_hex::<String>())
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn has_ibc_asset(&self, id: asset::Id) -> Result<bool> {
        match self
            .get_raw(&asset_storage_key(id))
            .await
            .context("failed reading raw asset from state")?
        {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    #[instrument(skip(self))]
    async fn get_ibc_asset(&self, id: asset::Id) -> Result<Denom> {
        let Some(bytes) = self
            .get_raw(&asset_storage_key(id))
            .await
            .context("failed reading raw asset from state")?
        else {
            bail!("asset not found");
        };

        let DenominationTrace(denom_str) =
            DenominationTrace::try_from_slice(&bytes).context("invalid asset bytes")?;
        let denom: Denom = denom_str.into();
        Ok(denom)
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_ibc_asset(&mut self, id: asset::Id, asset: &Denom) -> Result<()> {
        let bytes = borsh::to_vec(&DenominationTrace(asset.denomination_trace()))
            .context("failed to serialize asset")?;
        self.put_raw(asset_storage_key(id), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod test {
    use astria_core::primitive::v1::asset::{
        Denom,
        Id,
    };
    use cnidarium::StateDelta;

    use super::{
        StateReadExt as _,
        StateWriteExt as _,
    };

    #[tokio::test]
    async fn get_ibc_asset_non_existent() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let asset = Id::from_denom("asset");

        // gets for non existing assets fail
        state
            .get_ibc_asset(asset)
            .await
            .expect_err("gets for non existing ibc assets should fail");
    }

    #[tokio::test]
    async fn has_ibc_asset() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let denom = Denom::from_base_denom("asset");

        // non existing calls are ok for 'has'
        assert!(
            !state
                .has_ibc_asset(denom.id())
                .await
                .expect("'has' for non existing ibc assets should be ok"),
            "query for non existing asset should return false"
        );

        state
            .put_ibc_asset(denom.id(), &denom)
            .expect("putting ibc asset should not fail");

        // existing calls are ok for 'has'
        assert!(
            state
                .has_ibc_asset(denom.id())
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
        let denom = Denom::from_base_denom("asset");
        state
            .put_ibc_asset(denom.id(), &denom)
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state
                .get_ibc_asset(denom.id())
                .await
                .expect("an ibc asset was written and must exist inside the database"),
            denom,
            "stored ibc asset was not what was expected"
        );
    }

    #[tokio::test]
    async fn put_ibc_asset_complex() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write new
        let denom = Denom::from_base_denom("asset_0");
        state
            .put_ibc_asset(denom.id(), &denom)
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state
                .get_ibc_asset(denom.id())
                .await
                .expect("an ibc asset was written and must exist inside the database"),
            denom,
            "stored ibc asset was not what was expected"
        );

        // can write another without affecting original
        let denom_1 = Denom::from_base_denom("asset_1");
        state
            .put_ibc_asset(denom_1.id(), &denom_1)
            .expect("putting ibc asset should not fail");
        assert_eq!(
            state
                .get_ibc_asset(denom_1.id())
                .await
                .expect("an additional ibc asset was written and must exist inside the database"),
            denom_1,
            "additional ibc asset was not what was expected"
        );
        assert_eq!(
            state
                .get_ibc_asset(denom.id())
                .await
                .expect("an ibc asset was written and must exist inside the database"),
            denom,
            "original ibc asset was not what was expected"
        );
    }

    #[tokio::test]
    async fn put_ibc_asset_can_write_unrelated_ids_to_denoms() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write unrelated ids and denoms
        let id_key = Id::from_denom("asset_0");
        let denom = Denom::from_base_denom("asset_1");
        state
            .put_ibc_asset(id_key, &denom)
            .expect("putting ibc asset should not fail");

        // see that id key and denom's stored id differ
        assert_ne!(
            state
                .get_ibc_asset(id_key)
                .await
                .expect("an ibc asset was written and must exist inside the database")
                .id(),
            id_key,
            "stored ibc asset was not what was expected"
        );
    }
}
