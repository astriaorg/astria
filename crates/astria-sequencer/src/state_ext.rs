use anyhow::{
    bail,
    Context as _,
    Result,
};
use astria_core::sequencer::v1::asset;
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use futures::StreamExt as _;
use tendermint::Time;
use tracing::instrument;

const NATIVE_ASSET_KEY: &[u8] = b"nativeasset";
const BLOCK_FEES_PREFIX: &str = "block_fees/";
const FEE_ASSET_PREFIX: &str = "fee_asset/";

fn storage_version_by_height_key(height: u64) -> Vec<u8> {
    format!("storage_version/{height}").into()
}

fn block_fees_key(asset: asset::Id) -> Vec<u8> {
    format!("{BLOCK_FEES_PREFIX}{}", crate::utils::Hex(asset.as_ref())).into()
}

fn fee_asset_key(asset: asset::Id) -> Vec<u8> {
    format!("{FEE_ASSET_PREFIX}{}", crate::utils::Hex(asset.as_ref())).into()
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_chain_id(&self) -> Result<String> {
        let Some(bytes) = self
            .get_raw("chain_id")
            .await
            .context("failed to read raw chain_id from state")?
        else {
            bail!("chain id not found in state");
        };

        String::from_utf8(bytes).context("failed to parse chain id from raw bytes")
    }

    #[instrument(skip(self))]
    async fn get_revision_number(&self) -> Result<u64> {
        // this is used for chain upgrades, which we do not currently have.
        Ok(0)
    }

    #[instrument(skip(self))]
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

    #[instrument(skip(self))]
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

    #[instrument(skip(self))]
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

    #[instrument(skip(self))]
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

    #[instrument(skip(self))]
    async fn get_block_fees(&self) -> Result<Vec<(asset::Id, u128)>> {
        let mut fees: Vec<(asset::Id, u128)> = Vec::new();

        let mut stream =
            std::pin::pin!(self.nonverifiable_prefix_raw(BLOCK_FEES_PREFIX.as_bytes()));
        while let Some(Ok((key, value))) = stream.next().await {
            // if the key isn't of the form `block_fees/{asset_id}`, then we have a bug
            // in `put_block_fees`
            let id_str = key
                .strip_prefix(BLOCK_FEES_PREFIX.as_bytes())
                .expect("prefix must always be present");
            let id =
                asset::Id::try_from_slice(&hex::decode(id_str).expect("key must be hex encoded"))
                    .context("failed to parse asset id from hex key")?;

            let Ok(bytes): Result<[u8; 16], _> = value.try_into() else {
                bail!("failed turning raw block fees bytes into u128; not 16 bytes?");
            };

            fees.push((id, u128::from_be_bytes(bytes)));
        }

        Ok(fees)
    }

    #[instrument(skip(self))]
    async fn is_allowed_fee_asset(&self, asset: asset::Id) -> Result<bool> {
        Ok(self
            .nonverifiable_get_raw(&fee_asset_key(asset))
            .await
            .context("failed to read raw fee asset from state")?
            .is_some())
    }

    #[instrument(skip(self))]
    async fn get_allowed_fee_assets(&self) -> Result<Vec<asset::Id>> {
        let mut assets = Vec::new();

        let mut stream = std::pin::pin!(self.nonverifiable_prefix_raw(FEE_ASSET_PREFIX.as_bytes()));
        while let Some(Ok((key, _))) = stream.next().await {
            // if the key isn't of the form `fee_asset/{asset_id}`, then we have a bug
            // in `put_allowed_fee_asset`
            let id_str = key
                .strip_prefix(FEE_ASSET_PREFIX.as_bytes())
                .expect("prefix must always be present");
            let id =
                asset::Id::try_from_slice(&hex::decode(id_str).expect("key must be hex encoded"))
                    .context("failed to parse asset id from hex key")?;

            assets.push(id);
        }

        Ok(assets)
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_chain_id(&mut self, chain_id: String) {
        self.put_raw("chain_id".into(), chain_id.into_bytes());
    }

    #[instrument(skip(self))]
    fn put_block_height(&mut self, height: u64) {
        self.put_raw("block_height".into(), height.to_be_bytes().to_vec());
    }

    #[instrument(skip(self))]
    fn put_block_timestamp(&mut self, timestamp: Time) {
        self.put_raw("block_timestamp".into(), timestamp.to_rfc3339().into());
    }

    #[instrument(skip(self))]
    fn put_storage_version_by_height(&mut self, height: u64, version: u64) {
        self.nonverifiable_put_raw(
            storage_version_by_height_key(height),
            version.to_be_bytes().to_vec(),
        );
    }

    #[instrument(skip(self))]
    fn put_native_asset_denom(&mut self, denom: &str) {
        self.nonverifiable_put_raw(NATIVE_ASSET_KEY.to_vec(), denom.as_bytes().to_vec());
    }

    /// Adds `amount` to the block fees for `asset`.
    #[instrument(skip(self))]
    async fn get_and_increase_block_fees(&mut self, asset: asset::Id, amount: u128) -> Result<()> {
        let current_amount = self
            .nonverifiable_get_raw(&block_fees_key(asset))
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
            .checked_add(amount)
            .context("block fees overflowed u128")?;

        self.nonverifiable_put_raw(block_fees_key(asset), new_amount.to_be_bytes().to_vec());
        Ok(())
    }

    #[instrument(skip(self))]
    async fn clear_block_fees(&mut self) {
        let mut stream =
            std::pin::pin!(self.nonverifiable_prefix_raw(BLOCK_FEES_PREFIX.as_bytes()));
        while let Some(Ok((key, _))) = stream.next().await {
            self.nonverifiable_delete(key);
        }
    }

    #[instrument(skip(self))]
    fn put_allowed_fee_asset(&mut self, asset: asset::Id) {
        self.nonverifiable_put_raw(fee_asset_key(asset), vec![]);
    }

    #[instrument(skip(self))]
    fn delete_allowed_fee_asset(&mut self, asset: asset::Id) {
        self.nonverifiable_delete(fee_asset_key(asset));
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod test {
    use cnidarium::StateDelta;
    use tendermint::Time;

    use super::{
        StateReadExt as _,
        StateWriteExt as _,
    };

    #[tokio::test]
    async fn chain_id() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        state
            .get_chain_id()
            .await
            .expect_err("no chain ID should exist at first");

        // can write new
        let chain_id_orig = "test-chain-orig";
        state.put_chain_id(chain_id_orig.to_string());
        assert_eq!(
            state
                .get_chain_id()
                .await
                .expect("a chain ID was written and must exist inside the database"),
            chain_id_orig,
            "stored chain ID was not what was expected"
        );

        // can rewrite with new value
        let chain_id_update = "test-chain-update";
        state.put_chain_id(chain_id_update.to_string());
        assert_eq!(
            state
                .get_chain_id()
                .await
                .expect("a new chain ID was written and must exist inside the database"),
            chain_id_update,
            "updated chain ID was not what was expected"
        );
    }

    #[tokio::test]
    async fn revision_number() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // current impl just returns 'ok'
        assert_eq!(
            state
                .get_revision_number()
                .await
                .expect("getting the revision number should return 0u64"),
            0u64,
            "returned revision number should be 0u64"
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
        let asset = astria_core::sequencer::v1::asset::Id::from_denom("asset_0");
        let amount = 100u128;
        state
            .get_and_increase_block_fees(asset, amount)
            .await
            .unwrap();

        // holds expected
        let fee_balances_updated = state.get_block_fees().await.unwrap();
        assert_eq!(
            fee_balances_updated[0],
            (asset, amount),
            "fee balances are not what they were expected to be"
        );
    }

    #[tokio::test]
    async fn block_fee_read_and_increase_can_delete() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write
        let asset_first = astria_core::sequencer::v1::asset::Id::from_denom("asset_0");
        let asset_second = astria_core::sequencer::v1::asset::Id::from_denom("asset_1");
        let amount_first = 100u128;
        let amount_second = 200u128;

        state
            .get_and_increase_block_fees(asset_first, amount_first)
            .await
            .unwrap();
        state
            .get_and_increase_block_fees(asset_second, amount_second)
            .await
            .unwrap();
        // holds expected
        let mut fee_balances = state.get_block_fees().await.unwrap();
        fee_balances.sort_by(|a, b| a.1.cmp(&b.1));
        assert_eq!(
            fee_balances,
            vec![(asset_first, amount_first), (asset_second, amount_second)],
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
        let asset = astria_core::sequencer::v1::asset::Id::from_denom("asset_0");
        assert!(
            !state
                .is_allowed_fee_asset(asset)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to return false"
        );

        // existent fee assets return true
        state.put_allowed_fee_asset(asset);
        assert!(
            state
                .is_allowed_fee_asset(asset)
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
        let asset = astria_core::sequencer::v1::asset::Id::from_denom("asset_0");
        state.put_allowed_fee_asset(asset);
        assert!(
            state
                .is_allowed_fee_asset(asset)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );

        // see can get fee asset
        let assets = state.get_allowed_fee_assets().await.unwrap();
        assert_eq!(
            assets,
            vec![asset],
            "expected returned allowed fee assets to match what was written in"
        );

        // can delete
        state.delete_allowed_fee_asset(asset);

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
        let asset_first = astria_core::sequencer::v1::asset::Id::from_denom("asset_0");
        state.put_allowed_fee_asset(asset_first);
        assert!(
            state
                .is_allowed_fee_asset(asset_first)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
        let asset_second = astria_core::sequencer::v1::asset::Id::from_denom("asset_1");
        state.put_allowed_fee_asset(asset_second);
        assert!(
            state
                .is_allowed_fee_asset(asset_second)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
        let asset_third = astria_core::sequencer::v1::asset::Id::from_denom("asset_3");
        state.put_allowed_fee_asset(asset_third);
        assert!(
            state
                .is_allowed_fee_asset(asset_third)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );

        // can delete
        state.delete_allowed_fee_asset(asset_second);

        // see is deleted
        let mut assets = state.get_allowed_fee_assets().await.unwrap();
        assets.sort();
        assets.reverse(); // asset ids are hashes of the string input which the statement below is ordered by
        assert_eq!(
            assets,
            vec![asset_first, asset_third],
            "delete for allowed fee asset did not behave as expected"
        );
    }
}
