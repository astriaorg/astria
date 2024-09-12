use anyhow::{
    bail,
    ensure,
    Context as _,
    Result,
};
use astria_core::primitive::v1::{
    Address,
    Bech32m,
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

fn base_prefix_key() -> &'static str {
    "prefixes/base"
}

fn ibc_compat_prefix_key() -> &'static str {
    "prefixes/ibc-compat"
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    async fn ensure_base_prefix(&self, address: &Address<Bech32m>) -> anyhow::Result<()> {
        let prefix = self
            .get_base_prefix()
            .await
            .context("failed to read base prefix from state")?;
        ensure!(
            prefix == address.prefix(),
            "address has prefix `{}` but only `{prefix}` is permitted",
            address.prefix(),
        );
        Ok(())
    }

    async fn try_base_prefixed(&self, slice: &[u8]) -> anyhow::Result<Address> {
        let prefix = self
            .get_base_prefix()
            .await
            .context("failed to read base prefix from state")?;
        Address::builder()
            .slice(slice)
            .prefix(prefix)
            .try_build()
            .context("failed to construct address from byte slice and state-provided base prefix")
    }

    #[instrument(skip_all)]
    async fn get_base_prefix(&self) -> Result<String> {
        let Some(bytes) = self
            .get_raw(base_prefix_key())
            .await
            .context("failed reading address base prefix from state")?
        else {
            bail!("no base prefix found in state");
        };
        String::from_utf8(bytes).context("prefix retrieved from storage is not valid utf8")
    }

    #[instrument(skip_all)]
    async fn get_ibc_compat_prefix(&self) -> Result<String> {
        let Some(bytes) = self
            .get_raw(ibc_compat_prefix_key())
            .await
            .context("failed reading address ibc compat prefix from state")?
        else {
            bail!("no ibc compat prefix found in state")
        };
        String::from_utf8(bytes).context("prefix retrieved from storage is not valid utf8")
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_base_prefix(&mut self, prefix: &str) {
        self.put_raw(base_prefix_key().into(), prefix.into());
    }

    #[instrument(skip_all)]
    fn put_ibc_compat_prefix(&mut self, prefix: &str) {
        self.put_raw(ibc_compat_prefix_key().into(), prefix.into());
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod test {
    use cnidarium::StateDelta;

    use super::{
        StateReadExt as _,
        StateWriteExt as _,
    };

    #[tokio::test]
    async fn put_and_get_base_prefix() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_base_prefix("astria");
        assert_eq!("astria", &state.get_base_prefix().await.unwrap());
    }

    #[tokio::test]
    async fn put_and_get_ibc_compat_prefix() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_ibc_compat_prefix("astriacompat");
        assert_eq!(
            "astriacompat",
            &state.get_ibc_compat_prefix().await.unwrap()
        );
    }
}
