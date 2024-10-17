use astria_core::primitive::v1::{
    Address,
    Bech32m,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        bail,
        ensure,
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

use super::storage::{
    self,
    keys,
};
use crate::storage::StoredValue;

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    async fn ensure_base_prefix(&self, address: &Address<Bech32m>) -> Result<()> {
        let prefix = self
            .get_base_prefix()
            .await
            .wrap_err("failed to read base prefix from state")?;
        ensure!(
            prefix == address.prefix(),
            "address has prefix `{}` but only `{prefix}` is permitted",
            address.prefix(),
        );
        Ok(())
    }

    async fn try_base_prefixed(&self, slice: &[u8]) -> Result<Address> {
        let prefix = self
            .get_base_prefix()
            .await
            .wrap_err("failed to read base prefix from state")?;
        Address::builder()
            .slice(slice)
            .prefix(prefix)
            .try_build()
            .wrap_err("failed to construct address from byte slice and state-provided base prefix")
    }

    #[instrument(skip_all, err)]
    async fn get_base_prefix(&self) -> Result<String> {
        let Some(bytes) = self
            .get_raw(keys::BASE_PREFIX)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading address base prefix from state")?
        else {
            bail!("no base prefix found in state");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::AddressPrefix::try_from(value).map(String::from))
            .context("invalid base prefix bytes")
    }

    #[instrument(skip_all, err)]
    async fn get_ibc_compat_prefix(&self) -> Result<String> {
        let Some(bytes) = self
            .get_raw(keys::IBC_COMPAT_PREFIX)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading address ibc compat prefix from state")?
        else {
            bail!("no ibc compat prefix found in state")
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::AddressPrefix::try_from(value).map(String::from))
            .wrap_err("invalid ibc compat prefix bytes")
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_base_prefix(&mut self, prefix: String) -> Result<()> {
        let bytes = StoredValue::from(storage::AddressPrefix::from(prefix.as_str()))
            .serialize()
            .context("failed to serialize base prefix")?;
        self.put_raw(keys::BASE_PREFIX.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_ibc_compat_prefix(&mut self, prefix: String) -> Result<()> {
        let bytes = StoredValue::from(storage::AddressPrefix::from(prefix.as_str()))
            .serialize()
            .context("failed to serialize ibc-compat prefix")?;
        self.put_raw(keys::IBC_COMPAT_PREFIX.to_string(), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    #[tokio::test]
    async fn put_and_get_base_prefix() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta.put_base_prefix("astria".to_string()).unwrap();
        assert_eq!("astria", &state_delta.get_base_prefix().await.unwrap());
    }

    #[tokio::test]
    async fn put_and_get_ibc_compat_prefix() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_ibc_compat_prefix("astriacompat".to_string())
            .unwrap();
        assert_eq!(
            "astriacompat",
            &state_delta.get_ibc_compat_prefix().await.unwrap()
        );
    }
}
