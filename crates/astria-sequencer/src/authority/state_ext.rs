use std::collections::BTreeMap;

use astria_core::primitive::v1::ADDRESS_LEN;
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        bail,
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

use super::{
    storage::{
        self,
        keys,
    },
    ValidatorSet,
};
use crate::{
    accounts::AddressBytes,
    storage::StoredValue,
};

fn validator_name(key: &[u8]) -> String {
    format!("{}/{}", keys::VALIDATOR_NAMES_PREFIX, hex::encode(key))
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_sudo_address(&self) -> Result<[u8; ADDRESS_LEN]> {
        let Some(bytes) = self
            .get_raw(keys::SUDO)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw sudo key from state")?
        else {
            // return error because sudo key must be set
            bail!("sudo key not found");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::AddressBytes::try_from(value).map(<[u8; ADDRESS_LEN]>::from))
            .wrap_err("invalid sudo key bytes")
    }

    #[instrument(skip_all)]
    async fn get_validator_set(&self) -> Result<ValidatorSet> {
        let Some(bytes) = self
            .get_raw(keys::VALIDATOR_SET)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw validator set from state")?
        else {
            // return error because validator set must be set
            bail!("validator set not found")
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::ValidatorSet::try_from(value).map(ValidatorSet::from))
            .wrap_err("invalid validator set bytes")
    }

    #[instrument(skip_all)]
    async fn get_validator_updates(&self) -> Result<ValidatorSet> {
        let Some(bytes) = self
            .nonverifiable_get_raw(keys::VALIDATOR_UPDATES.as_bytes())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw validator updates from state")?
        else {
            // return empty set because validator updates are optional
            return Ok(ValidatorSet(BTreeMap::new()));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::ValidatorSet::try_from(value).map(ValidatorSet::from))
            .wrap_err("invalid validator update bytes")
    }

    #[instrument(skip_all)]
    async fn get_validator_name(&self, validator: &[u8]) -> Result<Option<String>> {
        let Some(bytes) = self
            .get_raw(&validator_name(validator))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw validator name from state")?
        else {
            return Ok(None);
        };
        Some(
            StoredValue::deserialize(&bytes)
                .and_then(|value| storage::ValidatorName::try_from(value).map(String::from))
                .wrap_err("invalid validator name bytes"),
        )
        .transpose()
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_sudo_address<T: AddressBytes>(&mut self, address: T) -> Result<()> {
        let bytes = StoredValue::from(storage::AddressBytes::from(&address))
            .serialize()
            .wrap_err("failed to serialize sudo address")?;
        self.put_raw(keys::SUDO.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_validator_set(&mut self, validator_set: ValidatorSet) -> Result<()> {
        let bytes = StoredValue::from(storage::ValidatorSet::from(&validator_set))
            .serialize()
            .wrap_err("failed to serialize validator set")?;
        self.put_raw(keys::VALIDATOR_SET.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_validator_updates(&mut self, validator_updates: ValidatorSet) -> Result<()> {
        let bytes = StoredValue::from(storage::ValidatorSet::from(&validator_updates))
            .serialize()
            .wrap_err("failed to serialize validator updates")?;
        self.nonverifiable_put_raw(keys::VALIDATOR_UPDATES.into(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn clear_validator_updates(&mut self) {
        self.nonverifiable_delete(keys::VALIDATOR_UPDATES.into());
    }

    #[instrument(skip_all)]
    fn remove_validator_name(&mut self, validator: &[u8]) {
        self.delete(validator_name(validator));
    }

    #[instrument(skip_all)]
    fn put_validator_name(&mut self, validator: &[u8], name: String) -> Result<()> {
        let bytes = StoredValue::from(storage::ValidatorName::from(name.as_str()))
            .serialize()
            .wrap_err("failed to serialize validator names")?;
        self.put_raw(validator_name(validator), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use astria_core::protocol::transaction::v1::action::ValidatorUpdate;
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        address::StateWriteExt as _,
        benchmark_and_test_utils::{
            verification_key,
            ASTRIA_PREFIX,
        },
    };

    fn empty_validator_set() -> ValidatorSet {
        ValidatorSet::new_from_updates(vec![])
    }

    #[tokio::test]
    async fn sudo_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        // doesn't exist at first
        let _ = state
            .get_sudo_address()
            .await
            .expect_err("no sudo address should exist at first");

        // can write new
        let mut address_expected = [42u8; ADDRESS_LEN];
        state
            .put_sudo_address(address_expected)
            .expect("writing sudo address should not fail");
        assert_eq!(
            state
                .get_sudo_address()
                .await
                .expect("a sudo address was written and must exist inside the database"),
            address_expected,
            "stored sudo address was not what was expected"
        );

        // can rewrite with new value
        address_expected = [41u8; ADDRESS_LEN];
        state
            .put_sudo_address(address_expected)
            .expect("writing sudo address should not fail");
        assert_eq!(
            state
                .get_sudo_address()
                .await
                .expect("a new sudo address was written and must exist inside the database"),
            address_expected,
            "updated sudo address was not what was expected"
        );
    }

    #[tokio::test]
    async fn validator_set_uninitialized_fails() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // doesn't exist at first
        let _ = state
            .get_validator_set()
            .await
            .expect_err("no validator set should exist at first");
    }

    #[tokio::test]
    async fn put_validator_set() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let initial = vec![ValidatorUpdate {
            power: 10,
            verification_key: verification_key(1),
            name: String::new(),
        }];
        let initial_validator_set = ValidatorSet::new_from_updates(initial);

        // can write new
        state
            .put_validator_set(initial_validator_set.clone())
            .expect("writing initial validator set should not fail");
        assert_eq!(
            state
                .get_validator_set()
                .await
                .expect("a validator set was written and must exist inside the database"),
            initial_validator_set,
            "stored validator set was not what was expected"
        );

        // can update
        let updates = vec![ValidatorUpdate {
            power: 20,
            verification_key: verification_key(2),
            name: String::new(),
        }];
        let updated_validator_set = ValidatorSet::new_from_updates(updates);
        state
            .put_validator_set(updated_validator_set.clone())
            .expect("writing update validator set should not fail");
        assert_eq!(
            state
                .get_validator_set()
                .await
                .expect("a validator set was written and must exist inside the database"),
            updated_validator_set,
            "stored validator set was not what was expected"
        );
    }

    #[tokio::test]
    async fn get_validator_updates_empty() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // querying for empty validator set is ok
        assert_eq!(
            state
                .get_validator_updates()
                .await
                .expect("if no updates have been written return empty set"),
            empty_validator_set(),
            "returned empty validator set different than expected"
        );
    }

    #[tokio::test]
    async fn put_validator_updates() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create update validator set
        let mut updates = vec![
            ValidatorUpdate {
                power: 10,
                verification_key: verification_key(1),
                name: String::new(),
            },
            ValidatorUpdate {
                power: 0,
                verification_key: verification_key(2),
                name: String::new(),
            },
        ];
        let mut validator_set_updates = ValidatorSet::new_from_updates(updates);

        // put validator updates
        state
            .put_validator_updates(validator_set_updates.clone())
            .expect("writing update validator set should not fail");
        assert_eq!(
            state
                .get_validator_updates()
                .await
                .expect("an update validator set was written and must exist inside the database"),
            validator_set_updates,
            "stored update validator set was not what was expected"
        );

        // create different updates
        updates = vec![
            ValidatorUpdate {
                power: 22,
                verification_key: verification_key(1),
                name: String::new(),
            },
            ValidatorUpdate {
                power: 10,
                verification_key: verification_key(3),
                name: String::new(),
            },
        ];

        validator_set_updates = ValidatorSet::new_from_updates(updates);

        // write different updates
        state
            .put_validator_updates(validator_set_updates.clone())
            .expect("writing update validator set should not fail");
        assert_eq!(
            state
                .get_validator_updates()
                .await
                .expect("an update validator set was written and must exist inside the database"),
            validator_set_updates,
            "stored update validator set was not what was expected"
        );
    }

    #[tokio::test]
    async fn clear_validator_updates() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create update validator set
        let updates = vec![ValidatorUpdate {
            power: 10,
            verification_key: verification_key(1),
            name: String::new(),
        }];
        let validator_set_updates = ValidatorSet::new_from_updates(updates);

        // put validator updates
        state
            .put_validator_updates(validator_set_updates.clone())
            .expect("writing update validator set should not fail");
        assert_eq!(
            state
                .get_validator_updates()
                .await
                .expect("an update validator set was written and must exist inside the database"),
            validator_set_updates,
            "stored update validator set was not what was expected"
        );

        // clear updates
        state.clear_validator_updates();

        // check that clear worked
        assert_eq!(
            state
                .get_validator_updates()
                .await
                .expect("if no updates have been written return empty set"),
            empty_validator_set(),
            "returned validator set different than expected"
        );
    }

    #[tokio::test]
    async fn clear_validator_updates_empty_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // able to clear non-existent updates with no error
        state.clear_validator_updates();
    }

    #[tokio::test]
    async fn execute_validator_updates() {
        // create initial validator set
        let initial = vec![
            ValidatorUpdate {
                power: 1,
                verification_key: verification_key(0),
                name: "test0".to_string(),
            },
            ValidatorUpdate {
                power: 2,
                verification_key: verification_key(1),
                name: "test1".to_string(),
            },
            ValidatorUpdate {
                power: 3,
                verification_key: verification_key(2),
                name: "test2".to_string(),
            },
        ];
        let mut initial_validator_set = ValidatorSet::new_from_updates(initial);

        // create set of updates (update key_0, remove key_1)
        let updates = vec![
            ValidatorUpdate {
                power: 5,
                verification_key: verification_key(0),
                name: "test0".to_string(),
            },
            ValidatorUpdate {
                power: 0,
                verification_key: verification_key(1),
                name: "test1".to_string(),
            },
        ];

        let validator_set_updates = ValidatorSet::new_from_updates(updates);

        // apply updates
        initial_validator_set.apply_updates(validator_set_updates);

        // create end state
        let updates = vec![
            ValidatorUpdate {
                power: 5,
                verification_key: verification_key(0),
                name: "test0".to_string(),
            },
            ValidatorUpdate {
                power: 3,
                verification_key: verification_key(2),
                name: "test2".to_string(),
            },
        ];
        let validator_set_endstate = ValidatorSet::new_from_updates(updates);

        // check updates applied correctly
        assert_eq!(
            initial_validator_set, validator_set_endstate,
            "validator set apply updates did not behave as expected"
        );
    }

    #[tokio::test]
    async fn put_and_get_validator_names() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let validator_address = &[0; ADDRESS_LEN];
        let validator_name = "ethan_was_here".to_string();

        // returns empty validator names if none in state
        assert_eq!(
            state.get_validator_name(validator_address).await.unwrap(),
            None
        );

        // can write new
        state
            .put_validator_name(validator_address, validator_name.clone())
            .expect("writing initial validator should not fail");
        assert_eq!(
            state
                .get_validator_name(validator_address)
                .await
                .expect("validator name was written and must exist inside the database"),
            Some(validator_name),
            "stored validator name was not what was expected"
        );

        // can update
        let validator_address_2 = &[1; ADDRESS_LEN];
        let validator_name_2 = "ethan_was_here_again".to_string();
        state
            .put_validator_name(validator_address_2, validator_name_2.clone())
            .expect("writing update validator set should not fail");
        assert_eq!(
            state
                .get_validator_name(validator_address_2)
                .await
                .expect("validator name was written and must exist inside the database"),
            Some(validator_name_2),
            "stored validator name was not what was expected"
        );
    }
}
