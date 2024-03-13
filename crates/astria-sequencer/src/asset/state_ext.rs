use anyhow::{
    bail,
    Context as _,
    Result,
};
use astria_core::sequencer::v1::{
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
