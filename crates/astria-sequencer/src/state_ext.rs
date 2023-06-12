use anyhow::{
    anyhow,
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
        let Some(bytes) = self.get_raw("block_height").await.context("failed to read raw block_height from state")? else {
            return Err(anyhow!("block height not found"))
        };
        let bytes: [u8; 8] = bytes
            .try_into()
            .map_err(|_| anyhow!("invalid block height"))?;
        Ok(u64::from_be_bytes(bytes))
    }

    #[instrument(skip(self))]
    async fn get_block_timestamp(&self) -> Result<Time> {
        let Some(bytes) = self.get_raw("block_timestamp").await.context("failed to read raw block_timestamp from state")? else {
            return Err(anyhow!("block timestamp not found"))
        };

        let timestamp = String::from_utf8(bytes)?;
        Ok(Time::parse_from_rfc3339(&timestamp)?)
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_block_height(&mut self, height: u64) {
        self.put_raw("block_height".into(), height.to_be_bytes().to_vec())
    }

    #[instrument(skip(self))]
    fn put_block_timestamp(&mut self, timestamp: Time) {
        self.put_raw("block_timestamp".into(), timestamp.to_rfc3339().into())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
