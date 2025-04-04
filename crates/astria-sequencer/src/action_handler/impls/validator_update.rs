use astria_core::{
    protocol::transaction::v1::action::ValidatorUpdate,
    upgrades::v1::{
        aspen::ValidatorUpdateActionChange,
        Aspen,
    },
};
use astria_eyre::eyre::{
    bail,
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::{
    instrument,
    Level,
};

use crate::{
    action_handler::ActionHandler,
    authority::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
    upgrades::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for ValidatorUpdate {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");

        if use_pre_aspen_validator_updates(&state)
            .await
            .wrap_err("failed to get upgrade status")?
        {
            // ensure that we're not removing the last validator or a validator
            // that doesn't exist, these both cause issues in cometBFT
            if self.power == 0 {
                let validator_set = state
                    ._pre_aspen_get_validator_set()
                    .await
                    .wrap_err("failed to get validator set from state")?;
                // check that validator exists
                if validator_set
                    .get(self.verification_key.address_bytes())
                    .is_none()
                {
                    bail!("cannot remove a non-existing validator");
                }
                // check that this is not the only validator, cannot remove the last one
                ensure!(validator_set.len() != 1, "cannot remove the last validator");
            }
        } else {
            let validator_count = state
                .get_validator_count()
                .await
                .wrap_err("failed to get validator count")?;
            if self.power == 0 {
                ensure!(validator_count > 1, "cannot remove the last validator",);
                state
                    .checked_remove_validator(self.verification_key.address_bytes())
                    .await
                    .wrap_err("failed to remove validator")?
                    .ok_or_eyre("cannot remove a non-existing validator")?;
                state.put_validator_count(validator_count.saturating_sub(1))?;
            } else {
                if state
                    .get_validator(self.verification_key.address_bytes())
                    .await
                    .wrap_err("failed to get validator")?
                    .is_none()
                {
                    state
                        .put_validator_count(validator_count.saturating_add(1))
                        .wrap_err("failed to put validator count")?;
                }
                state
                    .put_validator(self)
                    .wrap_err("failed to put validator in state")?;
            }
        }

        // add validator update in non-consensus state to be used in end_block
        let mut validator_updates = state
            .get_block_validator_updates()
            .await
            .wrap_err("failed getting validator updates from state")?;
        validator_updates.insert(self.clone());
        state
            .put_block_validator_updates(validator_updates)
            .wrap_err("failed to put validator updates in state")?;

        Ok(())
    }
}

pub(crate) async fn use_pre_aspen_validator_updates<S: StateRead>(state: &S) -> Result<bool> {
    let pre_aspen_upgrade = state
        .get_upgrade_change_info(&Aspen::NAME, &ValidatorUpdateActionChange::NAME)
        .await?
        .is_none();
    Ok(pre_aspen_upgrade)
}

#[cfg(test)]
mod tests {
    use astria_core::{
        crypto::VerificationKey,
        primitive::v1::TransactionId,
        upgrades::test_utils::UpgradesBuilder,
    };
    use futures::TryStreamExt;

    use super::*;
    use crate::{
        accounts::AddressBytes as _,
        authority::ValidatorSet,
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
        },
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
        upgrades::StateWriteExt,
    };

    #[tokio::test]
    async fn pre_aspen_validator_update_add_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let sudo_address = astria_address(&[0; 20]);
        state.put_sudo_address(sudo_address).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        assert_eq!(state.get_block_validator_updates().await.unwrap().len(), 0);

        let action = ValidatorUpdate {
            verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            power: 100,
            name: String::new(),
        };

        action.check_and_execute(&mut state).await.unwrap();

        let validator_updates = state.get_block_validator_updates().await.unwrap();
        assert_eq!(validator_updates.len(), 1);
        assert_eq!(
            validator_updates.get(action.verification_key.address_bytes()),
            Some(&action)
        );
    }

    #[tokio::test]
    async fn pre_aspen_validator_update_remove_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let sudo_address = astria_address(&[0; 20]);
        state.put_sudo_address(sudo_address).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let validator_update_1 = ValidatorUpdate {
            verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            power: 100,
            name: String::new(),
        };

        let validator_update_2 = ValidatorUpdate {
            verification_key: VerificationKey::try_from([1; 32]).unwrap(),
            power: 100,
            name: String::new(),
        };

        state
            ._pre_aspen_put_validator_set(ValidatorSet::new_from_updates(vec![
                validator_update_1.clone(),
                validator_update_2.clone(),
            ]))
            .unwrap();

        assert_eq!(state._pre_aspen_get_validator_set().await.unwrap().len(), 2);

        let action = ValidatorUpdate {
            verification_key: validator_update_1.verification_key,
            power: 0,
            name: String::new(),
        };

        action.check_and_execute(&mut state).await.unwrap();

        let validator_updates = state.get_block_validator_updates().await.unwrap();
        assert_eq!(validator_updates.len(), 1);
        assert_eq!(
            validator_updates.get(action.verification_key.address_bytes()),
            Some(&action)
        );
    }

    #[tokio::test]
    async fn validator_update_fails_if_signer_is_not_sudo_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_sudo_address(astria_address(&[1; 20])).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: [0; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = ValidatorUpdate {
            verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            power: 100,
            name: String::new(),
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "signer is not the sudo key",
        );
    }
    #[tokio::test]
    async fn pre_aspen_validator_update_remove_fails_if_validator_is_not_in_set() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let sudo_address = astria_address(&[0; 20]);
        state.put_sudo_address(sudo_address).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        state
            ._pre_aspen_put_validator_set(ValidatorSet::new_from_updates(vec![]))
            .unwrap();

        let action = ValidatorUpdate {
            verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            power: 0,
            name: String::new(),
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "cannot remove a non-existing validator",
        );
    }

    #[tokio::test]
    async fn pre_aspen_validator_update_remove_fails_if_attempting_to_remove_only_validator() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let sudo_address = astria_address(&[0; 20]);
        state.put_sudo_address(sudo_address).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let validator_update_1 = ValidatorUpdate {
            verification_key: VerificationKey::try_from([1; 32]).unwrap(),
            power: 100,
            name: String::new(),
        };

        state
            ._pre_aspen_put_validator_set(ValidatorSet::new_from_updates(vec![
                validator_update_1.clone()
            ]))
            .unwrap();

        let action = ValidatorUpdate {
            verification_key: validator_update_1.verification_key,
            power: 0,
            name: String::new(),
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "cannot remove the last validator",
        );
    }

    #[tokio::test]
    async fn post_aspen_validator_update_add_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state
            .put_upgrade_change_info(
                &Aspen::NAME,
                UpgradesBuilder::new()
                    .set_aspen(Some(1))
                    .build()
                    .aspen()
                    .unwrap()
                    .validator_update_action_change(),
            )
            .unwrap();

        let sudo_address = astria_address(&[0; 20]);
        state.put_sudo_address(sudo_address).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_validator_count(0).unwrap();

        assert_eq!(state.get_block_validator_updates().await.unwrap().len(), 0);

        let action = ValidatorUpdate {
            verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            power: 100,
            name: String::new(),
        };

        action.check_and_execute(&mut state).await.unwrap();

        // Check block validator updates
        let validator_updates = state.get_block_validator_updates().await.unwrap();
        assert_eq!(validator_updates.len(), 1);
        assert_eq!(
            validator_updates.get(action.verification_key.address_bytes()),
            Some(&action)
        );

        // Check state validators
        let validator_update = state
            .get_validator(action.verification_key.address_bytes())
            .await
            .unwrap()
            .expect("validator should be present");
        assert_eq!(validator_update, action);
        let validator_updates = state
            .get_validators()
            .try_collect::<Vec<ValidatorUpdate>>()
            .await
            .unwrap();
        assert_eq!(validator_updates.len(), 1);
        assert_eq!(validator_updates[0], action);

        // Check validator count
        let validator_count = state.get_validator_count().await.unwrap();
        assert_eq!(validator_count, 1);
    }

    #[tokio::test]
    async fn post_aspen_validator_update_remove_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state
            .put_upgrade_change_info(
                &Aspen::NAME,
                UpgradesBuilder::new()
                    .set_aspen(Some(1))
                    .build()
                    .aspen()
                    .unwrap()
                    .validator_update_action_change(),
            )
            .unwrap();

        let sudo_address = astria_address(&[0; 20]);
        state.put_sudo_address(sudo_address).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let validator_update_1 = ValidatorUpdate {
            verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            power: 100,
            name: String::new(),
        };
        state.put_validator(&validator_update_1).unwrap();

        let validator_update_2 = ValidatorUpdate {
            verification_key: VerificationKey::try_from([1; 32]).unwrap(),
            power: 100,
            name: String::new(),
        };
        state.put_validator(&validator_update_2).unwrap();
        state.put_validator_count(2).unwrap();

        // Check that validators are correctly set
        assert_eq!(
            state
                .get_validator(validator_update_1.verification_key.address_bytes())
                .await
                .unwrap()
                .unwrap(),
            validator_update_1
        );
        assert_eq!(
            state
                .get_validator(validator_update_2.verification_key.address_bytes())
                .await
                .unwrap()
                .unwrap(),
            validator_update_2
        );
        assert_eq!(
            state
                .get_validators()
                .try_collect::<Vec<ValidatorUpdate>>()
                .await
                .unwrap()
                .len(),
            2
        );

        let action = ValidatorUpdate {
            verification_key: validator_update_1.verification_key.clone(),
            power: 0,
            name: String::new(),
        };

        action.check_and_execute(&mut state).await.unwrap();

        // Check block validator updates
        let validator_updates = state.get_block_validator_updates().await.unwrap();
        assert_eq!(validator_updates.len(), 1);
        assert_eq!(
            validator_updates.get(action.verification_key.address_bytes()),
            Some(&action)
        );

        // Check state validators
        assert!(
            state
                .get_validator(validator_update_1.verification_key.address_bytes())
                .await
                .unwrap()
                .is_none(),
            "validator should be removed"
        );
        assert_eq!(
            state
                .get_validator(validator_update_2.verification_key.address_bytes())
                .await
                .unwrap()
                .unwrap(),
            validator_update_2
        );
        let validator_updates = state
            .get_validators()
            .try_collect::<Vec<ValidatorUpdate>>()
            .await
            .unwrap();
        assert_eq!(validator_updates.len(), 1);
        assert_eq!(validator_updates[0], validator_update_2);

        // Check validator count
        let validator_count = state.get_validator_count().await.unwrap();
        assert_eq!(validator_count, 1);
    }

    #[tokio::test]
    async fn post_aspen_validator_update_remove_fails_if_validator_does_not_exist() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state
            .put_upgrade_change_info(
                &Aspen::NAME,
                UpgradesBuilder::new()
                    .set_aspen(Some(1))
                    .build()
                    .aspen()
                    .unwrap()
                    .validator_update_action_change(),
            )
            .unwrap();

        let sudo_address = astria_address(&[0; 20]);
        state.put_sudo_address(sudo_address).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_validator_count(2).unwrap();

        let action = ValidatorUpdate {
            verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            power: 0,
            name: String::new(),
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "cannot remove a non-existing validator",
        );
    }

    #[tokio::test]
    async fn post_aspen_validator_update_remove_fails_if_attempting_to_remove_only_validator() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state
            .put_upgrade_change_info(
                &Aspen::NAME,
                UpgradesBuilder::new()
                    .set_aspen(Some(1))
                    .build()
                    .aspen()
                    .unwrap()
                    .validator_update_action_change(),
            )
            .unwrap();

        let sudo_address = astria_address(&[0; 20]);
        state.put_sudo_address(sudo_address).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let validator_update_1 = ValidatorUpdate {
            verification_key: VerificationKey::try_from([1; 32]).unwrap(),
            power: 100,
            name: String::new(),
        };
        state.put_validator_count(1).unwrap();

        let action = ValidatorUpdate {
            verification_key: validator_update_1.verification_key,
            power: 0,
            name: String::new(),
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "cannot remove the last validator",
        );
    }

    #[tokio::test]
    async fn post_aspen_validator_update_correctly_updates_existing_validator() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state
            .put_upgrade_change_info(
                &Aspen::NAME,
                UpgradesBuilder::new()
                    .set_aspen(Some(1))
                    .build()
                    .aspen()
                    .unwrap()
                    .validator_update_action_change(),
            )
            .unwrap();

        let sudo_address = astria_address(&[0; 20]);
        state.put_sudo_address(sudo_address).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let validator_update_1 = ValidatorUpdate {
            verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            power: 100,
            name: String::new(),
        };
        state.put_validator(&validator_update_1).unwrap();
        state.put_validator_count(1).unwrap();

        let action = ValidatorUpdate {
            verification_key: validator_update_1.verification_key.clone(),
            power: 200,
            name: String::new(),
        };

        action.check_and_execute(&mut state).await.unwrap();

        // Check block validator updates
        let validator_updates = state.get_block_validator_updates().await.unwrap();
        assert_eq!(validator_updates.len(), 1);
        assert_eq!(
            validator_updates.get(action.verification_key.address_bytes()),
            Some(&action)
        );

        // Check stored validator
        let validator_update = state
            .get_validator(action.verification_key.address_bytes())
            .await
            .unwrap()
            .expect("validator should be present");
        assert_eq!(validator_update, action);

        // Check validator count
        let validator_count = state.get_validator_count().await.unwrap();
        assert_eq!(validator_count, 1);
    }
}
