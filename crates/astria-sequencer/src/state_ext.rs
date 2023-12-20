use anyhow::{
    bail,
    Context as _,
    Result,
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tendermint::Time;
use tracing::instrument;

const NATIVE_ASSET_KEY: &[u8] = b"nativeasset";

fn storage_version_by_height_key(height: u64) -> Vec<u8> {
    format!("storage_version/{height}").into()
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
        // TODO: this is only for chain upgrades
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
}

impl<T: StateWrite> StateWriteExt for T {}
