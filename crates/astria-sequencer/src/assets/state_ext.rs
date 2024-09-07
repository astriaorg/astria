use anyhow::{
    bail,
    Context as _,
    Result,
};
use astria_core::primitive::v1::asset;
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use futures::StreamExt as _;
use tendermint::abci::{
    Event,
    EventAttributeIndexExt as _,
};
use tracing::instrument;

use crate::storage::{
    self,
    StoredValue,
};

const BLOCK_FEES_PREFIX: &str = "block_fees/";
const FEE_ASSET_PREFIX: &str = "fee_asset/";
const NATIVE_ASSET_KEY: &[u8] = b"nativeasset";

fn asset_storage_key<'a, TAsset>(asset: &'a TAsset) -> String
where
    asset::IbcPrefixed: From<&'a TAsset>,
{
    format!("asset/{}", crate::storage_keys::hunks::Asset::from(asset))
}

fn block_fees_key<'a, TAsset>(asset: &'a TAsset) -> String
where
    asset::IbcPrefixed: From<&'a TAsset>,
{
    format!(
        "{BLOCK_FEES_PREFIX}{}",
        crate::storage_keys::hunks::Asset::from(asset)
    )
}

fn fee_asset_key<'a, TAsset>(asset: &'a TAsset) -> String
where
    asset::IbcPrefixed: From<&'a TAsset>,
{
    format!(
        "{FEE_ASSET_PREFIX}{}",
        crate::storage_keys::hunks::Asset::from(asset)
    )
}

/// Creates `abci::Event` of kind `tx.fees` for sequencer fee reporting
fn construct_tx_fee_event<T: std::fmt::Display>(
    asset: &T,
    fee_amount: u128,
    action_type: String,
) -> Event {
    Event::new(
        "tx.fees",
        [
            ("asset", asset.to_string()).index(),
            ("feeAmount", fee_amount.to_string()).index(),
            ("actionType", action_type).index(),
        ],
    )
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_native_asset(&self) -> Result<asset::TracePrefixed> {
        let Some(bytes) = self
            .nonverifiable_get_raw(NATIVE_ASSET_KEY)
            .await
            .context("failed to read raw native asset from state")?
        else {
            bail!("native asset denom not found in state");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::TracePrefixedDenom::try_from(value).map(asset::TracePrefixed::from)
            })
            .context("invalid native asset bytes")
    }

    #[instrument(skip_all)]
    async fn has_ibc_asset<'a, TAsset>(&self, asset: &'a TAsset) -> Result<bool>
    where
        TAsset: Sync,
        asset::IbcPrefixed: From<&'a TAsset>,
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
        asset: &asset::IbcPrefixed,
    ) -> Result<Option<asset::TracePrefixed>> {
        let Some(bytes) = self
            .get_raw(&asset_storage_key(asset))
            .await
            .context("failed reading raw asset from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::TracePrefixedDenom::try_from(value)
                    .map(|stored_denom| Some(asset::TracePrefixed::from(stored_denom)))
            })
            .context("invalid ibc asset bytes")
    }

    #[instrument(skip_all)]
    async fn get_block_fees(&self) -> Result<Vec<(asset::IbcPrefixed, u128)>> {
        let mut fees = Vec::new();

        let mut stream =
            std::pin::pin!(self.nonverifiable_prefix_raw(BLOCK_FEES_PREFIX.as_bytes()));
        while let Some(Ok((key, bytes))) = stream.next().await {
            // if the key isn't of the form `block_fees/{asset_id}`, then we have a bug
            // in `put_block_fees`
            let suffix = key
                .strip_prefix(BLOCK_FEES_PREFIX.as_bytes())
                .expect("prefix must always be present");
            let asset = std::str::from_utf8(suffix)
                .context("key suffix was not utf8 encoded; this should not happen")?
                .parse::<crate::storage_keys::hunks::Asset>()
                .context("failed to parse storage key suffix as address hunk")?
                .get();

            let fee = StoredValue::deserialize(&bytes)
                .and_then(|value| storage::Fee::try_from(value).map(u128::from))
                .context("invalid block fees bytes")?;

            fees.push((asset, fee));
        }

        Ok(fees)
    }

    #[instrument(skip_all)]
    async fn is_allowed_fee_asset<'a, TAsset>(&self, asset: &'a TAsset) -> Result<bool>
    where
        TAsset: Sync,
        asset::IbcPrefixed: From<&'a TAsset>,
    {
        Ok(self
            .nonverifiable_get_raw(fee_asset_key(asset).as_bytes())
            .await
            .context("failed to read raw fee asset from state")?
            .is_some())
    }

    #[instrument(skip_all)]
    async fn get_allowed_fee_assets(&self) -> Result<Vec<asset::IbcPrefixed>> {
        let mut assets = Vec::new();

        let mut stream = std::pin::pin!(self.nonverifiable_prefix_raw(FEE_ASSET_PREFIX.as_bytes()));
        while let Some(Ok((key, _))) = stream.next().await {
            // if the key isn't of the form `fee_asset/{asset_id}`, then we have a bug
            // in `put_allowed_fee_asset`
            let suffix = key
                .strip_prefix(FEE_ASSET_PREFIX.as_bytes())
                .expect("prefix must always be present");
            let asset = std::str::from_utf8(suffix)
                .context("key suffix was not utf8 encoded; this should not happen")?
                .parse::<crate::storage_keys::hunks::Asset>()
                .context("failed to parse storage key suffix as address hunk")?
                .get();
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
        let bytes = StoredValue::TracePrefixedDenom((&asset).into())
            .serialize()
            .context("failed to serialize native asset")?;
        self.nonverifiable_put_raw(NATIVE_ASSET_KEY.to_vec(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_ibc_asset(&mut self, asset: asset::TracePrefixed) -> Result<()> {
        let key = asset_storage_key(&asset);
        let bytes = StoredValue::TracePrefixedDenom((&asset).into())
            .serialize()
            .context("failed to serialize ibc asset")?;
        self.put_raw(key, bytes);
        Ok(())
    }

    /// Adds `amount` to the block fees for `asset`.
    #[instrument(skip_all)]
    async fn get_and_increase_block_fees<'a, TAsset>(
        &mut self,
        asset: &'a TAsset,
        amount: u128,
        action_type: String,
    ) -> Result<()>
    where
        TAsset: Sync + std::fmt::Display,
        asset::IbcPrefixed: From<&'a TAsset>,
    {
        let tx_fee_event = construct_tx_fee_event(asset, amount, action_type);
        let block_fees_key = block_fees_key(asset);

        let current_amount = self
            .nonverifiable_get_raw(block_fees_key.as_bytes())
            .await
            .context("failed to read raw block fees from state")?
            .map(|bytes| {
                StoredValue::deserialize(&bytes)
                    .and_then(|value| storage::Fee::try_from(value).map(u128::from))
                    .context("invalid block fees bytes")
            })
            .transpose()?
            .unwrap_or_default();

        let new_amount = current_amount
            .checked_add(amount)
            .context("block fees overflowed u128")?;
        let bytes = StoredValue::Fee(new_amount.into())
            .serialize()
            .context("failed to serialize block fees")?;
        self.nonverifiable_put_raw(block_fees_key.into(), bytes);

        self.record(tx_fee_event);

        Ok(())
    }

    #[instrument(skip_all)]
    async fn clear_block_fees(&mut self) {
        let mut stream =
            std::pin::pin!(self.nonverifiable_prefix_raw(BLOCK_FEES_PREFIX.as_bytes()));
        while let Some(Ok((key, _))) = stream.next().await {
            self.nonverifiable_delete(key);
        }
    }

    #[instrument(skip_all)]
    fn delete_allowed_fee_asset<'a, TAsset>(&mut self, asset: &'a TAsset)
    where
        asset::IbcPrefixed: From<&'a TAsset>,
    {
        self.nonverifiable_delete(fee_asset_key(asset).into());
    }

    #[instrument(skip_all)]
    fn put_allowed_fee_asset<'a, TAsset>(&mut self, asset: &'a TAsset) -> Result<()>
    where
        asset::IbcPrefixed: From<&'a TAsset>,
    {
        let bytes = StoredValue::Unit
            .serialize()
            .context("failed to serialize unit for allowed fee asset")?;
        self.nonverifiable_put_raw(fee_asset_key(asset).into(), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use astria_core::primitive::v1::asset;
    use cnidarium::StateDelta;

    use super::{
        asset_storage_key,
        block_fees_key,
        fee_asset_key,
        StateReadExt as _,
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
    fn asset_2() -> asset::Denom {
        "asset_2".parse().unwrap()
    }

    #[tokio::test]
    async fn native_asset() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        state
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
    async fn block_fee_read_and_increase() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        let fee_balances_orig = state.get_block_fees().await.unwrap();
        assert!(fee_balances_orig.is_empty());

        // can write
        let asset = asset_0();
        let amount = 100u128;
        state
            .get_and_increase_block_fees(&asset, amount, "test".into())
            .await
            .unwrap();

        // holds expected
        let fee_balances_updated = state.get_block_fees().await.unwrap();
        assert_eq!(
            fee_balances_updated[0],
            (asset.to_ibc_prefixed(), amount),
            "fee balances are not what they were expected to be"
        );
    }

    #[tokio::test]
    async fn block_fee_read_and_increase_can_delete() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write
        let asset_first = asset_0();
        let asset_second = asset_1();
        let amount_first = 100u128;
        let amount_second = 200u128;

        state
            .get_and_increase_block_fees(&asset_first, amount_first, "test".into())
            .await
            .unwrap();
        state
            .get_and_increase_block_fees(&asset_second, amount_second, "test".into())
            .await
            .unwrap();
        // holds expected
        let fee_balances = HashSet::<_>::from_iter(state.get_block_fees().await.unwrap());
        assert_eq!(
            fee_balances,
            HashSet::from_iter(vec![
                (asset_first.to_ibc_prefixed(), amount_first),
                (asset_second.to_ibc_prefixed(), amount_second)
            ]),
            "returned fee balance vector not what was expected"
        );

        // can delete
        state.clear_block_fees().await;

        let fee_balances_updated = state.get_block_fees().await.unwrap();
        assert!(
            fee_balances_updated.is_empty(),
            "fee balances were expected to be deleted but were not"
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

    #[test]
    fn storage_keys_are_unchanged() {
        let asset = "an/asset/with/a/prefix"
            .parse::<astria_core::primitive::v1::asset::Denom>()
            .unwrap();
        assert_eq!(
            asset_storage_key(&asset),
            asset_storage_key(&asset.to_ibc_prefixed()),
        );
        insta::assert_snapshot!(asset_storage_key(&asset));

        let trace_prefixed = "a/denom/with/a/prefix"
            .parse::<astria_core::primitive::v1::asset::Denom>()
            .unwrap();
        assert_eq!(
            block_fees_key(&trace_prefixed),
            block_fees_key(&trace_prefixed.to_ibc_prefixed()),
        );
        insta::assert_snapshot!(block_fees_key(&trace_prefixed));

        assert_eq!(
            fee_asset_key(&trace_prefixed),
            fee_asset_key(&trace_prefixed.to_ibc_prefixed()),
        );
        insta::assert_snapshot!(fee_asset_key(&trace_prefixed));
    }
}
