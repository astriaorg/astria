use anyhow::{
    Context as _,
    Result,
};
use astria_core::{
    generated::sequencer::v1alpha1 as raw,
    sequencer::v1alpha1::block::SequencerBlock,
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use prost::Message;
use tracing::instrument;

const SEQUENCER_BLOCK_BY_HASH_PREFIX: &str = "block";
const SEQUENCER_BLOCK_HASH_BY_NUMBER_PREFIX: &str = "hash";

fn block_hash_by_height_key(height: u64) -> String {
    format!("{SEQUENCER_BLOCK_HASH_BY_NUMBER_PREFIX}/{height}")
}

fn sequencer_block_by_hash_key(hash: &[u8]) -> String {
    format!("{SEQUENCER_BLOCK_BY_HASH_PREFIX}/{}", hex::encode(hash))
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_block_hash_by_height(&self, height: u64) -> Result<Option<[u8; 32]>> {
        let key = block_hash_by_height_key(height);
        let hash = self
            .get_raw(&key)
            .await
            .context("failed to read block hash by height from state")?;
        match hash {
            Some(hash) => {
                let hash: [u8; 32] = hash
                    .as_slice()
                    .try_into()
                    .expect("block hash must be 32 bytes");
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn get_sequencer_block_by_hash(&self, hash: &[u8]) -> Result<Option<SequencerBlock>> {
        let key = sequencer_block_by_hash_key(hash);
        let bytes = self
            .get_raw(&key)
            .await
            .context("failed to read raw sequencer block from state")?;

        match bytes {
            Some(bytes) => {
                let raw = raw::SequencerBlock::decode::<bytes::Bytes>(bytes.into())
                    .context("failed to decode sequencer block from raw bytes")?;
                let block = SequencerBlock::try_from_raw(raw)
                    .context("failed to convert raw sequencer block to sequencer block")?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn get_sequencer_block_by_height(&self, height: u64) -> Result<Option<SequencerBlock>> {
        let hash = self
            .get_block_hash_by_height(height)
            .await
            .context("failed to get block hash by height")?;
        match hash {
            Some(hash) => self
                .get_sequencer_block_by_hash(&hash)
                .await
                .context("failed to get sequencer block by hash"),
            None => Ok(None),
        }
    }
}

pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_sequencer_block(&mut self, block: SequencerBlock) {
        let key = block_hash_by_height_key(block.height().into());
        self.put_raw(key, block.block_hash().to_vec());

        let key = sequencer_block_by_hash_key(&block.block_hash());
        let bytes = block.into_raw().encode_to_vec();
        self.put_raw(key, bytes);
    }
}
