use astria_core::protocol::transaction::v1::action::ValidatorUpdate;
use astria_eyre::eyre::{
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;
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

        // ensure that we're not removing the last validator or a validator
        // that doesn't exist, these both cause issues in cometBFT
        if self.power == 0 {
            let validator_set = state
                .get_validator_set()
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

        // add validator update in non-consensus state to be used in end_block
        let mut validator_updates = state
            .get_validator_updates()
            .await
            .wrap_err("failed getting validator updates from state")?;
        validator_updates.push_update(self.clone());
        state
            .put_validator_updates(validator_updates)
            .wrap_err("failed to put validator updates in state")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        crypto::VerificationKey,
        primitive::v1::TransactionId,
    };

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
    };

    #[tokio::test]
    async fn validator_update_add_executes_as_expected() {
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

        assert_eq!(state.get_validator_updates().await.unwrap().len(), 0);

        let action = ValidatorUpdate {
            verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            power: 100,
        };

        action.check_and_execute(&mut state).await.unwrap();

        let validator_updates = state.get_validator_updates().await.unwrap();
        assert_eq!(validator_updates.len(), 1);
        assert_eq!(
            validator_updates.get(action.verification_key.address_bytes()),
            Some(&action)
        );
    }

    #[tokio::test]
    async fn validator_update_remove_works_as_expected() {
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
        };

        let validator_update_2 = ValidatorUpdate {
            verification_key: VerificationKey::try_from([1; 32]).unwrap(),
            power: 100,
        };

        state
            .put_validator_set(ValidatorSet::new_from_updates(vec![
                validator_update_1.clone(),
                validator_update_2.clone(),
            ]))
            .unwrap();

        assert_eq!(state.get_validator_set().await.unwrap().len(), 2);

        let action = ValidatorUpdate {
            verification_key: validator_update_1.verification_key,
            power: 0,
        };

        action.check_and_execute(&mut state).await.unwrap();

        let validator_updates = state.get_validator_updates().await.unwrap();
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
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "signer is not the sudo key",
        );
    }
    #[tokio::test]
    async fn validator_update_remove_fails_if_validator_is_not_in_set() {
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
            .put_validator_set(ValidatorSet::new_from_updates(vec![]))
            .unwrap();

        let action = ValidatorUpdate {
            verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            power: 0,
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "cannot remove a non-existing validator",
        );
    }

    #[tokio::test]
    async fn validator_update_remove_fails_if_attempting_to_remove_only_validator() {
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
        };

        state
            .put_validator_set(ValidatorSet::new_from_updates(vec![
                validator_update_1.clone()
            ]))
            .unwrap();

        let action = ValidatorUpdate {
            verification_key: validator_update_1.verification_key,
            power: 0,
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "cannot remove the last validator",
        );
    }
}
