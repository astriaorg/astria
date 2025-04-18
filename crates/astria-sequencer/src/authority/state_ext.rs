use std::{
    collections::BTreeMap,
    pin::Pin,
    task::{
        ready,
        Context,
        Poll,
    },
};

use astria_core::{
    primitive::v1::ADDRESS_LEN,
    protocol::transaction::v1::action::ValidatorUpdate,
};
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
use futures::Stream;
use pin_project_lite::pin_project;
use tracing::{
    instrument,
    Level,
};

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

pin_project! {
    /// A stream of all existing validators in state.
    pub(crate) struct ValidatorStream<St> {
        #[pin]
        underlying: St,
    }
}

impl<St> Stream for ValidatorStream<St>
where
    St: Stream<Item = astria_eyre::anyhow::Result<(String, Vec<u8>)>>,
{
    type Item = Result<ValidatorUpdate>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let (_, bytes) = match ready!(this.underlying.as_mut().poll_next(cx)) {
            Some(Ok(tup)) => tup,
            Some(Err(err)) => {
                return Poll::Ready(Some(Err(
                    anyhow_to_eyre(err).wrap_err("failed reading from state")
                )));
            }
            None => return Poll::Ready(None),
        };
        let update = StoredValue::deserialize(&bytes)
            .and_then(|value| storage::ValidatorInfoV1::try_from(value).map(ValidatorUpdate::from))
            .wrap_err("invalid validator info bytes")?;
        Poll::Ready(Some(Ok(update)))
    }
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all, err(level = Level::WARN))]
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

    #[instrument(skip_all, err(level = Level::WARN))]
    async fn get_block_validator_updates(&self) -> Result<ValidatorSet> {
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

    #[instrument(skip_all, err(level = Level::WARN))]
    async fn get_validator<TAddress: AddressBytes>(
        &self,
        validator: &TAddress,
    ) -> Result<Option<ValidatorUpdate>> {
        let Some(bytes) = self
            .get_raw(&keys::validator(validator))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw validator info from state")?
        else {
            return Ok(None);
        };
        Some(
            StoredValue::deserialize(&bytes)
                .and_then(|value| {
                    storage::ValidatorInfoV1::try_from(value).map(ValidatorUpdate::from)
                })
                .wrap_err("invalid validator info bytes"),
        )
        .transpose()
    }

    #[instrument(skip_all, err(level = Level::WARN))]
    async fn get_validator_count(&self) -> Result<u64> {
        let Some(bytes) = self
            .nonverifiable_get_raw(keys::VALIDATOR_COUNT.as_bytes())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw validator count from state")?
        else {
            bail!("validator count not found in state");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::ValidatorCount::try_from(value).map(u64::from))
            .wrap_err("invalid validator count bytes")
    }

    #[instrument(skip_all)]
    fn get_validators(&self) -> ValidatorStream<Self::PrefixRawStream> {
        ValidatorStream {
            underlying: self.prefix_raw(keys::VALIDATOR_PREFIX),
        }
    }

    /// Deprecated as of Aspen upgrade
    #[instrument(skip_all, err(level = Level::WARN))]
    async fn pre_aspen_get_validator_set(&self) -> Result<ValidatorSet> {
        let Some(bytes) = self
            .get_raw(keys::PRE_ASPEN_VALIDATOR_SET)
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
    fn put_block_validator_updates(&mut self, validator_updates: ValidatorSet) -> Result<()> {
        let bytes = StoredValue::from(storage::ValidatorSet::from(&validator_updates))
            .serialize()
            .wrap_err("failed to serialize validator updates")?;
        self.nonverifiable_put_raw(keys::VALIDATOR_UPDATES.into(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn clear_block_validator_updates(&mut self) {
        self.nonverifiable_delete(keys::VALIDATOR_UPDATES.into());
    }

    #[instrument(skip_all)]
    async fn remove_validator<TAddress: AddressBytes>(
        &mut self,
        validator_address: &TAddress,
    ) -> Result<bool> {
        if self
            .get_raw(&keys::validator(validator_address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw validator info from state")?
            .is_none()
        {
            return Ok(false);
        };
        self.delete(keys::validator(validator_address));
        Ok(true)
    }

    #[instrument(skip_all)]
    fn put_validator(&mut self, validator: &ValidatorUpdate) -> Result<()> {
        let bytes = StoredValue::from(storage::ValidatorInfoV1::from(validator))
            .serialize()
            .wrap_err("failed to serialize validator update")?;
        self.put_raw(keys::validator(&validator.verification_key), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_validator_count(&mut self, count: u64) -> Result<()> {
        let bytes = StoredValue::from(storage::ValidatorCount::from(count))
            .serialize()
            .wrap_err("failed to serialize validator count")?;
        self.nonverifiable_put_raw(keys::VALIDATOR_COUNT.as_bytes().to_vec(), bytes);
        Ok(())
    }

    /// Deprecated as of Aspen upgrade
    #[instrument(skip_all)]
    fn pre_aspen_put_validator_set(&mut self, validator_set: ValidatorSet) -> Result<()> {
        let bytes = StoredValue::from(storage::ValidatorSet::from(&validator_set))
            .serialize()
            .wrap_err("failed to serialize validator set")?;
        self.put_raw(keys::PRE_ASPEN_VALIDATOR_SET.to_string(), bytes);
        Ok(())
    }

    /// Should only be called ONCE, at Aspen upgrade
    #[instrument(skip_all)]
    fn aspen_upgrade_remove_validator_set(&mut self) -> Result<()> {
        self.delete(keys::PRE_ASPEN_VALIDATOR_SET.to_string());
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use astria_core::protocol::transaction::v1::action::{
        ValidatorName,
        ValidatorUpdate,
    };
    use cnidarium::StateDelta;
    use futures::TryStreamExt as _;

    use super::*;
    use crate::benchmark_and_test_utils::verification_key;

    fn empty_validator_set() -> ValidatorSet {
        ValidatorSet::new_from_updates(vec![])
    }

    #[tokio::test]
    async fn sudo_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

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
    async fn pre_aspen_validator_set_uninitialized_fails() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // doesn't exist at first
        let _ = state
            .pre_aspen_get_validator_set()
            .await
            .expect_err("no validator set should exist at first");
    }

    #[tokio::test]
    async fn put_get_and_remove_pre_aspen_validator_set() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let initial = vec![ValidatorUpdate {
            power: 10,
            verification_key: verification_key(1),
            name: ValidatorName::empty(),
        }];
        let initial_validator_set = ValidatorSet::new_from_updates(initial);

        // can write new
        state
            .pre_aspen_put_validator_set(initial_validator_set.clone())
            .expect("writing initial validator set should not fail");
        assert_eq!(
            state
                .pre_aspen_get_validator_set()
                .await
                .expect("a validator set was written and must exist inside the database"),
            initial_validator_set,
            "stored validator set was not what was expected"
        );

        // can update
        let updates = vec![ValidatorUpdate {
            power: 20,
            verification_key: verification_key(2),
            name: ValidatorName::empty(),
        }];
        let updated_validator_set = ValidatorSet::new_from_updates(updates);
        state
            .pre_aspen_put_validator_set(updated_validator_set.clone())
            .expect("writing update validator set should not fail");
        assert_eq!(
            state
                .pre_aspen_get_validator_set()
                .await
                .expect("a validator set was written and must exist inside the database"),
            updated_validator_set,
            "stored validator set was not what was expected"
        );

        // can remove
        state
            .aspen_upgrade_remove_validator_set()
            .expect("removing validator set should not fail");
        let _ = state
            .pre_aspen_get_validator_set()
            .await
            .expect_err("no validator set should exist at first");
    }

    #[tokio::test]
    async fn get_block_validator_updates_empty() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // querying for empty validator set is ok
        assert_eq!(
            state
                .get_block_validator_updates()
                .await
                .expect("if no updates have been written return empty set"),
            empty_validator_set(),
            "returned empty validator set different than expected"
        );
    }

    #[tokio::test]
    async fn put_block_validator_updates() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create update validator set
        let mut updates = vec![
            ValidatorUpdate {
                power: 10,
                verification_key: verification_key(1),
                name: ValidatorName::empty(),
            },
            ValidatorUpdate {
                power: 0,
                verification_key: verification_key(2),
                name: ValidatorName::empty(),
            },
        ];
        let mut validator_set_updates = ValidatorSet::new_from_updates(updates);

        // put validator updates
        state
            .put_block_validator_updates(validator_set_updates.clone())
            .expect("writing update validator set should not fail");
        assert_eq!(
            state
                .get_block_validator_updates()
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
                name: ValidatorName::empty(),
            },
            ValidatorUpdate {
                power: 10,
                verification_key: verification_key(3),
                name: ValidatorName::empty(),
            },
        ];

        validator_set_updates = ValidatorSet::new_from_updates(updates);

        // write different updates
        state
            .put_block_validator_updates(validator_set_updates.clone())
            .expect("writing update validator set should not fail");
        assert_eq!(
            state
                .get_block_validator_updates()
                .await
                .expect("an update validator set was written and must exist inside the database"),
            validator_set_updates,
            "stored update validator set was not what was expected"
        );
    }

    #[tokio::test]
    async fn clear_block_validator_updates() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create update validator set
        let updates = vec![ValidatorUpdate {
            power: 10,
            verification_key: verification_key(1),
            name: ValidatorName::empty(),
        }];
        let validator_set_updates = ValidatorSet::new_from_updates(updates);

        // put validator updates
        state
            .put_block_validator_updates(validator_set_updates.clone())
            .expect("writing update validator set should not fail");
        assert_eq!(
            state
                .get_block_validator_updates()
                .await
                .expect("an update validator set was written and must exist inside the database"),
            validator_set_updates,
            "stored update validator set was not what was expected"
        );

        // clear updates
        state.clear_block_validator_updates();

        // check that clear worked
        assert_eq!(
            state
                .get_block_validator_updates()
                .await
                .expect("if no updates have been written return empty set"),
            empty_validator_set(),
            "returned validator set different than expected"
        );
    }

    #[tokio::test]
    async fn clear_block_validator_updates_empty_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // able to clear non-existent updates with no error
        state.clear_block_validator_updates();
    }

    #[tokio::test]
    async fn execute_pre_aspen_validator_updates() {
        // create initial validator set
        let initial = vec![
            ValidatorUpdate {
                power: 1,
                verification_key: verification_key(0),
                name: "test0".parse().unwrap(),
            },
            ValidatorUpdate {
                power: 2,
                verification_key: verification_key(1),
                name: "test1".parse().unwrap(),
            },
            ValidatorUpdate {
                power: 3,
                verification_key: verification_key(2),
                name: "test2".parse().unwrap(),
            },
        ];
        let mut initial_validator_set = ValidatorSet::new_from_updates(initial);

        // create set of updates (update key_0, remove key_1)
        let updates = vec![
            ValidatorUpdate {
                power: 5,
                verification_key: verification_key(0),
                name: "test0".parse().unwrap(),
            },
            ValidatorUpdate {
                power: 0,
                verification_key: verification_key(1),
                name: "test1".parse().unwrap(),
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
                name: "test0".parse().unwrap(),
            },
            ValidatorUpdate {
                power: 3,
                verification_key: verification_key(2),
                name: "test2".parse().unwrap(),
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
    async fn put_and_get_validator_count() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        let _ = state
            .get_validator_count()
            .await
            .expect_err("no validator count should exist at first");

        // can write new
        let mut count_expected = 8;
        state
            .put_validator_count(count_expected)
            .expect("writing validator count should not fail");
        assert_eq!(
            state
                .get_validator_count()
                .await
                .expect("a validator count was written and must exist inside the database"),
            count_expected,
            "stored validator count was not what was expected"
        );

        // can rewrite with new value
        count_expected = 7;
        state
            .put_validator_count(count_expected)
            .expect("writing validator count should not fail");
        assert_eq!(
            state
                .get_validator_count()
                .await
                .expect("a new validator count was written and must exist inside the database"),
            count_expected,
            "updated validator count was not what was expected"
        );
    }

    #[expect(
        clippy::too_many_lines,
        reason = "I think test length is warranted to test full functionality without splitting \
                  into many confusingly similar tests"
    )]
    #[tokio::test]
    async fn put_get_and_remove_validator() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let validator_1 = ValidatorUpdate {
            power: 10,
            verification_key: verification_key(1),
            name: "test1".parse().unwrap(),
        };
        let mut validator_2 = ValidatorUpdate {
            power: 20,
            verification_key: verification_key(2),
            name: "test2".parse().unwrap(),
        };

        // No validator returns `None`
        assert!(
            state
                .get_validator(validator_1.verification_key.address_bytes())
                .await
                .unwrap()
                .is_none(),
            "validator should not exist yet"
        );
        assert!(
            state
                .get_validator(validator_2.verification_key.address_bytes())
                .await
                .unwrap()
                .is_none(),
            "validator should not exist yet"
        );

        // can write new
        state
            .put_validator(&validator_1)
            .expect("writing validator 1 should not fail");
        state
            .put_validator(&validator_2)
            .expect("writing validator 2 should not fail");

        // check that both validators exist
        assert_eq!(
            state
                .get_validator(validator_1.verification_key.address_bytes())
                .await
                .expect("a validator was written and must exist inside the database"),
            Some(validator_1.clone()),
            "stored validator 1 was not what was expected"
        );
        assert_eq!(
            state
                .get_validator(validator_2.verification_key.address_bytes())
                .await
                .expect("a validator was written and must exist inside the database"),
            Some(validator_2.clone()),
            "stored validator 2 was not what was expected"
        );

        // can rewrite with new value
        validator_2.power = 30;
        state
            .put_validator(&validator_2)
            .expect("writing validator 2 should not fail");
        assert_eq!(
            state
                .get_validator(validator_2.verification_key.address_bytes())
                .await
                .expect("a new validator 2 was written and must exist inside the database"),
            Some(validator_2.clone()),
            "updated validator 2 was not what was expected"
        );

        // validator stream works as expected
        let mut validators = state
            .get_validators()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert_eq!(
            validators.len(),
            2,
            "validator stream should return 2 validators"
        );
        assert!(
            validators.contains(&validator_1),
            "validator stream should contain validator 1"
        );
        assert!(
            validators.contains(&validator_2),
            "validator stream should contain validator 2"
        );

        // remove validator works as expected
        assert!(
            state
                .remove_validator(validator_1.verification_key.address_bytes())
                .await
                .expect("removing validator 1 should not fail"),
            "validator 1 should exist prior to removal"
        );
        // trying to remove again returns false
        assert!(
            !state
                .remove_validator(validator_1.verification_key.address_bytes())
                .await
                .expect("removing validator 1 should not fail"),
            "validator 1 should not exist after removal"
        );

        assert!(
            state
                .get_validator(validator_1.verification_key.address_bytes())
                .await
                .unwrap()
                .is_none(),
            "validator 1 should not exist anymore"
        );
        validators = state
            .get_validators()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert_eq!(
            validators.len(),
            1,
            "validator stream should return 1 validator"
        );
        assert_eq!(
            validators[0], validator_2,
            "validator stream should contain validator 2"
        );
    }
}
