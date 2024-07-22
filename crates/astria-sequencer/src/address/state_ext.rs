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
use tracing::instrument;

fn base_prefix_key() -> &'static str {
    "prefixes/base"
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_base_prefix(&self) -> Result<String> {
        let Some(bytes) = self
            .get_raw(base_prefix_key())
            .await
            .context("failed reading address base prefix")?
        else {
            bail!("no base prefix found");
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
}
