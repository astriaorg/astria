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
use tendermint::{
    abci::{
        Event,
        EventAttributeIndexExt as _,
    },
    Time,
};
use tracing::instrument;

const NATIVE_ASSET_KEY: &[u8] = b"nativeasset";
const REVISION_NUMBER_KEY: &str = "revision_number";
const BLOCK_FEES_PREFIX: &str = "block_fees/";
const FEE_ASSET_PREFIX: &str = "fee_asset/";

fn storage_version_by_height_key(height: u64) -> Vec<u8> {
    format!("storage_version/{height}").into()
}

fn block_fees_key<TAsset: Into<asset::IbcPrefixed>>(asset: TAsset) -> String {
    format!(
        "{BLOCK_FEES_PREFIX}{}",
        crate::storage_keys::hunks::Asset::from(asset)
    )
}

fn fee_asset_key<TAsset: Into<asset::IbcPrefixed>>(asset: TAsset) -> String {
    format!(
        "{FEE_ASSET_PREFIX}{}",
        crate::storage_keys::hunks::Asset::from(asset)
    )
}

/// Creates `abci::Event` of kind `tx.fees` for sequencer fee reporting
fn construct_tx_fee_event<TAsset>(asset: &TAsset, fee_amount: u128, action_type: String) -> Event
where
    TAsset: Into<asset::IbcPrefixed> + std::fmt::Display + Send,
{
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
    async fn get_chain_id(&self) -> Result<tendermint::chain::Id> {
        let Some(bytes) = self
            .get_raw("chain_id")
            .await
            .context("failed to read raw chain_id from state")?
        else {
            bail!("chain id not found in state");
        };

        Ok(String::from_utf8(bytes)
            .context("failed to parse chain id from raw bytes")?
            .try_into()
            .expect("only valid chain ids should be stored in the state"))
    }

    #[instrument(skip_all)]
    async fn get_revision_number(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(REVISION_NUMBER_KEY)
            .await
            .context("failed to read raw revision number from state")?
        else {
            bail!("revision number not found in state");
        };

        let bytes = TryInto::<[u8; 8]>::try_into(bytes).map_err(|b| {
            anyhow::anyhow!(
                "expected 8 revision number bytes but got {}; this is a bug",
                b.len()
            )
        })?;

        Ok(u64::from_be_bytes(bytes))
    }

    #[instrument(skip_all)]
    async fn get_block_height(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw("block_height")
            .await
            .context("failed to read raw block_height from state")?
        else {
            bail!("block height not found state");
        };
        let Ok(bytes): Result<[u8; 8], _> = bytes.try_into() else {
            bail!("failed turning raw block height bytes into u64; not 8 bytes?");
        };
        Ok(u64::from_be_bytes(bytes))
    }

    #[instrument(skip_all)]
    async fn get_block_timestamp(&self) -> Result<Time> {
        let Some(bytes) = self
            .get_raw("block_timestamp")
            .await
            .context("failed to read raw block_timestamp from state")?
        else {
            bail!("block timestamp not found");
        };
        // no extra allocations in the happy path (meaning the bytes are utf8)
        Time::parse_from_rfc3339(&String::from_utf8_lossy(&bytes))
            .context("failed to parse timestamp from raw timestamp bytes")
    }

    #[instrument(skip_all)]
    async fn get_storage_version_by_height(&self, height: u64) -> Result<u64> {
        let key = storage_version_by_height_key(height);
        let Some(bytes) = self
            .nonverifiable_get_raw(&key)
            .await
            .context("failed to read raw storage_version from state")?
        else {
            bail!("storage version not found");
        };
        let Ok(bytes): Result<[u8; 8], _> = bytes.try_into() else {
            bail!("failed turning raw storage version bytes into u64; not 8 bytes?");
        };
        Ok(u64::from_be_bytes(bytes))
    }

    #[instrument(skip_all)]
    async fn get_native_asset_denom(&self) -> Result<String> {
        let Some(bytes) = self
            .nonverifiable_get_raw(NATIVE_ASSET_KEY)
            .await
            .context("failed to read raw native_asset_denom from state")?
        else {
            bail!("native asset denom not found");
        };

        String::from_utf8(bytes).context("failed to parse native asset denom from raw bytes")
    }

    #[instrument(skip_all)]
    async fn get_block_fees(&self) -> Result<Vec<(asset::IbcPrefixed, u128)>> {
        // let mut fees: Vec<(asset::Id, u128)> = Vec::new();
        let mut fees = Vec::new();

        let mut stream =
            std::pin::pin!(self.nonverifiable_prefix_raw(BLOCK_FEES_PREFIX.as_bytes()));
        while let Some(Ok((key, value))) = stream.next().await {
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

            let Ok(bytes): Result<[u8; 16], _> = value.try_into() else {
                bail!("failed turning raw block fees bytes into u128; not 16 bytes?");
            };

            fees.push((asset, u128::from_be_bytes(bytes)));
        }

        Ok(fees)
    }

    #[instrument(skip_all)]
    async fn is_allowed_fee_asset<TAsset>(&self, asset: TAsset) -> Result<bool>
    where
        TAsset: Into<asset::IbcPrefixed> + std::fmt::Display + Send,
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

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_chain_id_and_revision_number(&mut self, chain_id: tendermint::chain::Id) {
        let revision_number = revision_number_from_chain_id(chain_id.as_str());
        self.put_raw("chain_id".into(), chain_id.as_bytes().to_vec());
        self.put_revision_number(revision_number);
    }

    #[instrument(skip_all)]
    fn put_revision_number(&mut self, revision_number: u64) {
        self.put_raw(
            REVISION_NUMBER_KEY.into(),
            revision_number.to_be_bytes().to_vec(),
        );
    }

    #[instrument(skip_all)]
    fn put_block_height(&mut self, height: u64) {
        self.put_raw("block_height".into(), height.to_be_bytes().to_vec());
    }

    #[instrument(skip_all)]
    fn put_block_timestamp(&mut self, timestamp: Time) {
        self.put_raw("block_timestamp".into(), timestamp.to_rfc3339().into());
    }

    #[instrument(skip_all)]
    fn put_storage_version_by_height(&mut self, height: u64, version: u64) {
        self.nonverifiable_put_raw(
            storage_version_by_height_key(height),
            version.to_be_bytes().to_vec(),
        );
    }

    #[instrument(skip_all)]
    fn put_native_asset_denom(&mut self, denom: &str) {
        self.nonverifiable_put_raw(NATIVE_ASSET_KEY.to_vec(), denom.as_bytes().to_vec());
    }

    /// Adds `amount` to the block fees for `asset`.
    #[instrument(skip_all)]
    async fn get_and_increase_block_fees<TAsset>(
        &mut self,
        asset: TAsset,
        fee_amount: u128,
        action_type: String,
    ) -> Result<()>
    where
        TAsset: Into<asset::IbcPrefixed> + std::fmt::Display + Send + Clone,
    {
        let block_fees_key = block_fees_key(asset.clone());
        let current_amount = self
            .nonverifiable_get_raw(block_fees_key.as_bytes())
            .await
            .context("failed to read raw block fees from state")?
            .map(|bytes| {
                let Ok(bytes): Result<[u8; 16], _> = bytes.try_into() else {
                    // this shouldn't happen
                    bail!("failed turning raw block fees bytes into u128; not 16 bytes?");
                };
                Ok(u128::from_be_bytes(bytes))
            })
            .transpose()?
            .unwrap_or_default();

        let new_amount = current_amount
            .checked_add(fee_amount)
            .context("block fees overflowed u128")?;
        self.nonverifiable_put_raw(block_fees_key.into(), new_amount.to_be_bytes().to_vec());

        // record the fee event to the state cache
        let tx_fee_event = construct_tx_fee_event(&asset, fee_amount, action_type);
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
    fn put_allowed_fee_asset<TAsset>(&mut self, asset: TAsset)
    where
        TAsset: Into<asset::IbcPrefixed> + std::fmt::Display + Send,
    {
        self.nonverifiable_put_raw(fee_asset_key(asset).into(), vec![]);
    }

    #[instrument(skip_all)]
    fn delete_allowed_fee_asset<TAsset>(&mut self, asset: TAsset)
    where
        TAsset: Into<asset::IbcPrefixed> + std::fmt::Display,
    {
        self.nonverifiable_delete(fee_asset_key(asset).into());
    }
}

impl<T: StateWrite> StateWriteExt for T {}

fn revision_number_from_chain_id(chain_id: &str) -> u64 {
    let re = regex::Regex::new(r".*-([0-9]+)$").unwrap();

    if !re.is_match(chain_id) {
        tracing::debug!("no revision number found in chain id; setting to 0");
        return 0;
    }

    let (_, revision_number): (&str, [&str; 1]) = re
        .captures(chain_id)
        .expect("should have a matching string")
        .extract();
    revision_number[0]
        .parse::<u64>()
        .expect("revision number must be parseable and fit in a u64")
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use cnidarium::StateDelta;
    use tendermint::Time;

    use super::{
        revision_number_from_chain_id,
        StateReadExt as _,
        StateWriteExt as _,
    };
    use crate::state_ext::{
        block_fees_key,
        fee_asset_key,
    };

    fn asset_0() -> astria_core::primitive::v1::asset::Denom {
        "asset_0".parse().unwrap()
    }
    fn asset_1() -> astria_core::primitive::v1::asset::Denom {
        "asset_1".parse().unwrap()
    }
    fn asset_2() -> astria_core::primitive::v1::asset::Denom {
        "asset_2".parse().unwrap()
    }

    #[test]
    fn revision_number_from_chain_id_regex() {
        let revision_number = revision_number_from_chain_id("test-chain-1024-99");
        assert_eq!(revision_number, 99u64);

        let revision_number = revision_number_from_chain_id("test-chain-1024");
        assert_eq!(revision_number, 1024u64);

        let revision_number = revision_number_from_chain_id("test-chain");
        assert_eq!(revision_number, 0u64);

        let revision_number = revision_number_from_chain_id("99");
        assert_eq!(revision_number, 0u64);

        let revision_number = revision_number_from_chain_id("99-1024");
        assert_eq!(revision_number, 1024u64);

        let revision_number = revision_number_from_chain_id("test-chain-1024-99-");
        assert_eq!(revision_number, 0u64);
    }

    #[tokio::test]
    async fn put_chain_id_and_revision_number() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        state
            .get_chain_id()
            .await
            .expect_err("no chain ID should exist at first");

        // can write new
        let chain_id_orig: tendermint::chain::Id = "test-chain-orig".try_into().unwrap();
        state.put_chain_id_and_revision_number(chain_id_orig.clone());
        assert_eq!(
            state
                .get_chain_id()
                .await
                .expect("a chain ID was written and must exist inside the database"),
            chain_id_orig,
            "stored chain ID was not what was expected"
        );

        assert_eq!(
            state
                .get_revision_number()
                .await
                .expect("getting the revision number should succeed"),
            0u64,
            "returned revision number should be 0u64 as chain id did not have a revision number"
        );

        // can rewrite with new value
        let chain_id_update: tendermint::chain::Id = "test-chain-update".try_into().unwrap();
        state.put_chain_id_and_revision_number(chain_id_update.clone());
        assert_eq!(
            state
                .get_chain_id()
                .await
                .expect("a new chain ID was written and must exist inside the database"),
            chain_id_update,
            "updated chain ID was not what was expected"
        );

        assert_eq!(
            state
                .get_revision_number()
                .await
                .expect("getting the revision number should succeed"),
            0u64,
            "returned revision number should be 0u64 as chain id did not have a revision number"
        );

        // can rewrite with chain id with revision number
        let chain_id_update: tendermint::chain::Id = "test-chain-99".try_into().unwrap();
        state.put_chain_id_and_revision_number(chain_id_update.clone());
        assert_eq!(
            state
                .get_chain_id()
                .await
                .expect("a new chain ID was written and must exist inside the database"),
            chain_id_update,
            "updated chain ID was not what was expected"
        );

        assert_eq!(
            state
                .get_revision_number()
                .await
                .expect("getting the revision number should succeed"),
            99u64,
            "returned revision number should be 0u64 as chain id did not have a revision number"
        );
    }

    #[tokio::test]
    async fn block_height() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        state
            .get_block_height()
            .await
            .expect_err("no block height should exist at first");

        // can write new
        let block_height_orig = 0;
        state.put_block_height(block_height_orig);
        assert_eq!(
            state
                .get_block_height()
                .await
                .expect("a block height was written and must exist inside the database"),
            block_height_orig,
            "stored block height was not what was expected"
        );

        // can rewrite with new value
        let block_height_update = 1;
        state.put_block_height(block_height_update);
        assert_eq!(
            state
                .get_block_height()
                .await
                .expect("a new block height was written and must exist inside the database"),
            block_height_update,
            "updated block height was not what was expected"
        );
    }

    #[tokio::test]
    async fn block_timestamp() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        state
            .get_block_timestamp()
            .await
            .expect_err("no block timestamp should exist at first");

        // can write new
        let block_timestamp_orig = Time::from_unix_timestamp(1_577_836_800, 0).unwrap();
        state.put_block_timestamp(block_timestamp_orig);
        assert_eq!(
            state
                .get_block_timestamp()
                .await
                .expect("a block timestamp was written and must exist inside the database"),
            block_timestamp_orig,
            "stored block timestamp was not what was expected"
        );

        // can rewrite with new value
        let block_timestamp_update = Time::from_unix_timestamp(1_577_836_801, 0).unwrap();
        state.put_block_timestamp(block_timestamp_update);
        assert_eq!(
            state
                .get_block_timestamp()
                .await
                .expect("a new block timestamp was written and must exist inside the database"),
            block_timestamp_update,
            "updated block timestamp was not what was expected"
        );
    }

    #[tokio::test]
    async fn storage_version() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        let block_height_orig = 0;
        state
            .get_storage_version_by_height(block_height_orig)
            .await
            .expect_err("no block height should exist at first");

        // can write for block height 0
        let storage_version_orig = 0;
        state.put_storage_version_by_height(block_height_orig, storage_version_orig);
        assert_eq!(
            state
                .get_storage_version_by_height(block_height_orig)
                .await
                .expect("a storage version was written and must exist inside the database"),
            storage_version_orig,
            "stored storage version was not what was expected"
        );

        // can update block height 0
        let storage_version_update = 0;
        state.put_storage_version_by_height(block_height_orig, storage_version_update);
        assert_eq!(
            state
                .get_storage_version_by_height(block_height_orig)
                .await
                .expect("a new storage version was written and must exist inside the database"),
            storage_version_update,
            "updated storage version was not what was expected"
        );

        // can write block 1 and block 0 is unchanged
        let block_height_update = 1;
        state.put_storage_version_by_height(block_height_update, storage_version_orig);
        assert_eq!(
            state
                .get_storage_version_by_height(block_height_update)
                .await
                .expect("a second storage version was written and must exist inside the database"),
            storage_version_orig,
            "additional storage version was not what was expected"
        );
        assert_eq!(
            state
                .get_storage_version_by_height(block_height_orig)
                .await
                .expect(
                    "the first storage version was written and should still exist inside the \
                     database"
                ),
            storage_version_update,
            "original but updated storage version was not what was expected"
        );
    }

    #[tokio::test]
    async fn native_asset_denom() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        state
            .get_native_asset_denom()
            .await
            .expect_err("no native asset denom should exist at first");

        // can write
        let denom_orig = "denom_orig";
        state.put_native_asset_denom(denom_orig);
        assert_eq!(
            state.get_native_asset_denom().await.expect(
                "a native asset denomination was written and must exist inside the database"
            ),
            denom_orig,
            "stored native asset denomination was not what was expected"
        );

        // can write new value
        let denom_update = "denom_update";
        state.put_native_asset_denom(denom_update);
        assert_eq!(
            state.get_native_asset_denom().await.expect(
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
        state.put_allowed_fee_asset(&asset);
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
        state.put_allowed_fee_asset(&asset);
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
        state.put_allowed_fee_asset(&asset_first);
        assert!(
            state
                .is_allowed_fee_asset(&asset_first)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
        let asset_second = asset_1();
        state.put_allowed_fee_asset(&asset_second);
        assert!(
            state
                .is_allowed_fee_asset(&asset_second)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
        let asset_third = asset_2();
        state.put_allowed_fee_asset(&asset_third);
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
    fn storage_keys_are_not_changed() {
        let trace_prefixed = "a/denom/with/a/prefix"
            .parse::<astria_core::primitive::v1::asset::Denom>()
            .unwrap();
        assert_eq!(
            block_fees_key(&trace_prefixed),
            block_fees_key(trace_prefixed.to_ibc_prefixed()),
        );
        insta::assert_snapshot!(block_fees_key(&trace_prefixed));

        assert_eq!(
            fee_asset_key(&trace_prefixed),
            fee_asset_key(trace_prefixed.to_ibc_prefixed()),
        );
        insta::assert_snapshot!(fee_asset_key(trace_prefixed));
    }
}
