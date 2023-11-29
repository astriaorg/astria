use anyhow::{
    anyhow,
    Context,
    Result,
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use hex::ToHex as _;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use proto::native::sequencer::v1alpha1::{
    asset,
    asset::IbcAsset,
};
use tracing::instrument;

/// Newtype wrapper to read and write a denomination trace from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct DenominationTrace(String);

const ASSET_PREFIX: &str = "asset";

fn asset_storage_key(asset: asset::Id) -> String {
    format!("{}/{}", ASSET_PREFIX, asset.encode_hex::<String>())
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_ibc_asset(&self, id: asset::Id) -> Result<IbcAsset> {
        let Some(bytes) = self
            .get_raw(&asset_storage_key(id))
            .await
            .context("failed reading raw asset from state")?
        else {
            return Err(anyhow!("asset not found"));
        };
        let DenominationTrace(asset) =
            DenominationTrace::try_from_slice(&bytes).context("invalid asset bytes")?;
        let asset: IbcAsset = asset.parse().context("invalid asset denomination")?;
        Ok(asset)
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_ibc_asset(&mut self, asset: IbcAsset) -> Result<()> {
        let bytes = DenominationTrace(asset.denomination_trace())
            .try_to_vec()
            .context("failed to serialize asset")?;
        self.put_raw(asset_storage_key(asset.id()), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
