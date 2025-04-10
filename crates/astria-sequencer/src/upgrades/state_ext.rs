use astria_core::upgrades::v1::{
    Change,
    ChangeInfo,
    ChangeName,
    UpgradeName,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
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
    #[instrument(skip_all)]
    async fn get_upgrade_change_info(
        &self,
        upgrade_name: &UpgradeName,
        change_name: &ChangeName,
    ) -> Result<Option<ChangeInfo>> {
        let Some(bytes) = self
            .get_raw(&keys::change(upgrade_name, change_name))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading change info from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::ChangeInfo::try_from(value).map(|info| Some(ChangeInfo::from(info)))
            })
            .wrap_err("invalid change info bytes")
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_upgrade_change_info(
        &mut self,
        upgrade_name: &UpgradeName,
        change: &dyn Change,
    ) -> Result<()> {
        let change_info = change.info();
        let bytes = StoredValue::from(storage::ChangeInfo::from(&change_info))
            .serialize()
            .wrap_err("failed to serialize change info")?;
        self.put_raw(keys::change(upgrade_name, &change.name()), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use cnidarium::StateDelta;

    use super::*;

    const UPGRADE_1: UpgradeName = UpgradeName::new("up one");

    #[derive(borsh::BorshSerialize)]
    struct TestChange;

    impl Change for TestChange {
        fn name(&self) -> ChangeName {
            ChangeName::from("test_change")
        }

        fn activation_height(&self) -> u64 {
            10
        }

        fn app_version(&self) -> u64 {
            2
        }
    }

    #[tokio::test]
    async fn change_info_roundtrip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        assert!(state
            .get_upgrade_change_info(&UPGRADE_1, &TestChange.name())
            .await
            .unwrap()
            .is_none());

        state
            .put_upgrade_change_info(&UPGRADE_1, &TestChange)
            .unwrap();

        assert_eq!(
            state
                .get_upgrade_change_info(&UPGRADE_1, &TestChange.name())
                .await
                .unwrap(),
            Some(TestChange.info()),
        );
    }
}
