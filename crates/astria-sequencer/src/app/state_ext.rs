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
use tendermint::Time;
use tracing::instrument;

use super::storage::{
    self,
    keys,
};
use crate::storage::StoredValue;

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_chain_id(&self) -> Result<tendermint::chain::Id> {
        let Some(bytes) = self
            .get_raw(keys::CHAIN_ID)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read raw chain_id from state")?
        else {
            bail!("chain id not found in state");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::ChainId::try_from(value).map(tendermint::chain::Id::from))
            .wrap_err("invalid chain id bytes")
    }

    #[instrument(skip_all)]
    async fn get_revision_number(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(keys::REVISION_NUMBER)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read raw revision number from state")?
        else {
            bail!("revision number not found in state");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::RevisionNumber::try_from(value).map(u64::from))
            .wrap_err("invalid revision number bytes")
    }

    #[instrument(skip_all)]
    async fn get_block_height(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(keys::BLOCK_HEIGHT)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read raw block_height from state")?
        else {
            bail!("block height not found state");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::BlockHeight::try_from(value).map(u64::from))
            .context("invalid block height bytes")
    }

    #[instrument(skip_all)]
    async fn get_block_timestamp(&self) -> Result<Time> {
        let Some(bytes) = self
            .get_raw(keys::BLOCK_TIMESTAMP)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read raw block_timestamp from state")?
        else {
            bail!("block timestamp not found");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::BlockTimestamp::try_from(value).map(Time::from))
            .wrap_err("invalid block timestamp bytes")
    }

    #[instrument(skip_all)]
    async fn get_storage_version_by_height(&self, height: u64) -> Result<u64> {
        use astria_eyre::eyre::WrapErr as _;

        let key = keys::storage_version_by_height(height);
        let Some(bytes) = self
            .nonverifiable_get_raw(key.as_bytes())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read raw storage_version from state")?
        else {
            bail!("storage version not found");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::StorageVersion::try_from(value).map(u64::from))
            .context("invalid storage version bytes")
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_chain_id_and_revision_number(&mut self, chain_id: tendermint::chain::Id) -> Result<()> {
        let revision_number = revision_number_from_chain_id(chain_id.as_str());
        let bytes = StoredValue::from(storage::ChainId::from(&chain_id))
            .serialize()
            .context("failed to serialize chain id")?;
        self.put_raw(keys::CHAIN_ID.into(), bytes);
        self.put_revision_number(revision_number)
    }

    #[instrument(skip_all)]
    fn put_revision_number(&mut self, revision_number: u64) -> Result<()> {
        let bytes = StoredValue::from(storage::RevisionNumber::from(revision_number))
            .serialize()
            .context("failed to serialize revision number")?;
        self.put_raw(keys::REVISION_NUMBER.into(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_block_height(&mut self, height: u64) -> Result<()> {
        let bytes = StoredValue::from(storage::BlockHeight::from(height))
            .serialize()
            .context("failed to serialize block height")?;
        self.put_raw(keys::BLOCK_HEIGHT.into(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_block_timestamp(&mut self, timestamp: Time) -> Result<()> {
        let bytes = StoredValue::from(storage::BlockTimestamp::from(timestamp))
            .serialize()
            .context("failed to serialize block timestamp")?;
        self.put_raw(keys::BLOCK_TIMESTAMP.into(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_storage_version_by_height(&mut self, height: u64, version: u64) -> Result<()> {
        let bytes = StoredValue::from(storage::StorageVersion::from(version))
            .serialize()
            .context("failed to serialize storage version")?;
        self.nonverifiable_put_raw(keys::storage_version_by_height(height).into_bytes(), bytes);
        Ok(())
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
mod tests {
    use super::*;
    use crate::storage::Storage;

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
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // doesn't exist at first
        let _ = state_delta
            .get_chain_id()
            .await
            .expect_err("no chain ID should exist at first");

        // can write new
        let chain_id_orig: tendermint::chain::Id = "test-chain-orig".try_into().unwrap();
        state_delta
            .put_chain_id_and_revision_number(chain_id_orig.clone())
            .unwrap();
        assert_eq!(
            state_delta
                .get_chain_id()
                .await
                .expect("a chain ID was written and must exist inside the database"),
            chain_id_orig,
            "stored chain ID was not what was expected"
        );

        assert_eq!(
            state_delta
                .get_revision_number()
                .await
                .expect("getting the revision number should succeed"),
            0u64,
            "returned revision number should be 0u64 as chain id did not have a revision number"
        );

        // can rewrite with new value
        let chain_id_update: tendermint::chain::Id = "test-chain-update".try_into().unwrap();
        state_delta
            .put_chain_id_and_revision_number(chain_id_update.clone())
            .unwrap();
        assert_eq!(
            state_delta
                .get_chain_id()
                .await
                .expect("a new chain ID was written and must exist inside the database"),
            chain_id_update,
            "updated chain ID was not what was expected"
        );

        assert_eq!(
            state_delta
                .get_revision_number()
                .await
                .expect("getting the revision number should succeed"),
            0u64,
            "returned revision number should be 0u64 as chain id did not have a revision number"
        );

        // can rewrite with chain id with revision number
        let chain_id_update: tendermint::chain::Id = "test-chain-99".try_into().unwrap();
        state_delta
            .put_chain_id_and_revision_number(chain_id_update.clone())
            .unwrap();
        assert_eq!(
            state_delta
                .get_chain_id()
                .await
                .expect("a new chain ID was written and must exist inside the database"),
            chain_id_update,
            "updated chain ID was not what was expected"
        );

        assert_eq!(
            state_delta
                .get_revision_number()
                .await
                .expect("getting the revision number should succeed"),
            99u64,
            "returned revision number should be 0u64 as chain id did not have a revision number"
        );
    }

    #[tokio::test]
    async fn block_height() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // doesn't exist at first
        let _ = state_delta
            .get_block_height()
            .await
            .expect_err("no block height should exist at first");

        // can write new
        let block_height_orig = 0;
        state_delta.put_block_height(block_height_orig).unwrap();
        assert_eq!(
            state_delta
                .get_block_height()
                .await
                .expect("a block height was written and must exist inside the database"),
            block_height_orig,
            "stored block height was not what was expected"
        );

        // can rewrite with new value
        let block_height_update = 1;
        state_delta.put_block_height(block_height_update).unwrap();
        assert_eq!(
            state_delta
                .get_block_height()
                .await
                .expect("a new block height was written and must exist inside the database"),
            block_height_update,
            "updated block height was not what was expected"
        );
    }

    #[tokio::test]
    async fn block_timestamp() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // doesn't exist at first
        let _ = state_delta
            .get_block_timestamp()
            .await
            .expect_err("no block timestamp should exist at first");

        // can write new
        let block_timestamp_orig = Time::from_unix_timestamp(1_577_836_800, 0).unwrap();
        state_delta
            .put_block_timestamp(block_timestamp_orig)
            .unwrap();
        assert_eq!(
            state_delta
                .get_block_timestamp()
                .await
                .expect("a block timestamp was written and must exist inside the database"),
            block_timestamp_orig,
            "stored block timestamp was not what was expected"
        );

        // can rewrite with new value
        let block_timestamp_update = Time::from_unix_timestamp(1_577_836_801, 0).unwrap();
        state_delta
            .put_block_timestamp(block_timestamp_update)
            .unwrap();
        assert_eq!(
            state_delta
                .get_block_timestamp()
                .await
                .expect("a new block timestamp was written and must exist inside the database"),
            block_timestamp_update,
            "updated block timestamp was not what was expected"
        );
    }

    #[tokio::test]
    async fn storage_version() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // doesn't exist at first
        let block_height_orig = 0;
        let _ = state_delta
            .get_storage_version_by_height(block_height_orig)
            .await
            .expect_err("no block height should exist at first");

        // can write for block height 0
        let storage_version_orig = 0;
        state_delta
            .put_storage_version_by_height(block_height_orig, storage_version_orig)
            .unwrap();
        assert_eq!(
            state_delta
                .get_storage_version_by_height(block_height_orig)
                .await
                .expect("a storage version was written and must exist inside the database"),
            storage_version_orig,
            "stored storage version was not what was expected"
        );

        // can update block height 0
        let storage_version_update = 0;
        state_delta
            .put_storage_version_by_height(block_height_orig, storage_version_update)
            .unwrap();
        assert_eq!(
            state_delta
                .get_storage_version_by_height(block_height_orig)
                .await
                .expect("a new storage version was written and must exist inside the database"),
            storage_version_update,
            "updated storage version was not what was expected"
        );

        // can write block 1 and block 0 is unchanged
        let block_height_update = 1;
        state_delta
            .put_storage_version_by_height(block_height_update, storage_version_orig)
            .unwrap();
        assert_eq!(
            state_delta
                .get_storage_version_by_height(block_height_update)
                .await
                .expect("a second storage version was written and must exist inside the database"),
            storage_version_orig,
            "additional storage version was not what was expected"
        );
        assert_eq!(
            state_delta
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
}
