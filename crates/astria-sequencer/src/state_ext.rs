use anyhow::{
    bail,
    Context as _,
    Result,
};
use async_trait::async_trait;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use tendermint::Time;
use tracing::instrument;

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
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
        let key: Vec<u8> = format!("storage_version/{height}").into();
        let Some(bytes) = self
            .nonconsensus_get_raw(key.as_slice())
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
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
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
        self.nonconsensus_put_raw(
            format!("storage_version/{height}").into(),
            version.to_be_bytes().to_vec(),
        );
    }
}

impl<T: StateWrite> StateWriteExt for T {}
