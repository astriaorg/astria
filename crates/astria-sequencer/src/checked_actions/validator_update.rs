use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::ValidatorUpdate,
    upgrades::v1::{
        aspen::ValidatorUpdateActionChange,
        Aspen,
    },
};
use astria_eyre::eyre::{
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::{
    debug,
    instrument,
    Level,
};

use super::{
    AssetTransfer,
    TransactionSignerAddressBytes,
};
use crate::{
    accounts::AddressBytes as _,
    authority::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    upgrades::StateReadExt as _,
};

#[derive(Debug)]
pub(crate) struct CheckedValidatorUpdate {
    action: ValidatorUpdate,
    tx_signer: TransactionSignerAddressBytes,
}

impl CheckedValidatorUpdate {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn new<S: StateRead>(
        action: ValidatorUpdate,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self> {
        let checked_action = Self {
            action,
            tx_signer: tx_signer.into(),
        };
        checked_action.run_mutable_checks(state).await?;

        Ok(checked_action)
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn run_mutable_checks<S: StateRead>(&self, state: S) -> Result<()> {
        self.do_run_mutable_checks(state).await.map(|_| ())
    }

    async fn do_run_mutable_checks<S: StateRead>(&self, state: S) -> Result<Option<Metadata>> {
        // Check that the signer of this tx is the authorized sudo address.
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to read sudo address from storage")?;
        ensure!(
            &sudo_address == self.tx_signer.as_bytes(),
            "transaction signer not authorized to update validator set",
        );

        if use_pre_aspen_validator_updates(&state)
            .await
            .wrap_err("failed to get upgrade status")?
        {
            // Ensure that we're not removing the last validator or a validator that doesn't exist;
            // these both cause issues in CometBFT.
            if self.action.power == 0 {
                let validator_set = state
                    .pre_aspen_get_validator_set()
                    .await
                    .wrap_err("failed to read validator set from storage")?;
                // Check that validator exists.
                if validator_set.get(&self.action.verification_key).is_none() {
                    bail!("cannot remove a non-existing validator");
                }
                // Check that this is not the only validator, cannot remove the last one.
                ensure!(validator_set.len() != 1, "cannot remove the only validator");
            }
            return Ok(None);
        }

        let current_validator_count = state
            .get_validator_count()
            .await
            .wrap_err("failed to read validator count from storage")?;
        let validator_already_exists = state
            .get_validator(&self.action.verification_key)
            .await
            .wrap_err("failed to read validator info from storage")?
            .is_some();
        if self.action.power == 0 {
            ensure!(
                current_validator_count > 1,
                "cannot remove the only validator"
            );
            ensure!(
                validator_already_exists,
                "cannot remove a non-existing validator"
            );
        }

        Ok(Some(Metadata {
            current_validator_count,
            validator_already_exists,
        }))
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        if let Some(metadata) = self.do_run_mutable_checks(&state).await? {
            if self.action.power == 0 {
                state.remove_validator(&self.action.verification_key).await;
                state
                    .put_validator_count(metadata.current_validator_count.saturating_sub(1))
                    .wrap_err("failed to write validator count to storage")?;
                debug!(
                    address = %self.action.verification_key.display_address(),
                    "removed validator"
                );
            } else {
                let log_msg = if metadata.validator_already_exists {
                    "updated validator"
                } else {
                    state
                        .put_validator_count(metadata.current_validator_count.saturating_add(1))
                        .wrap_err("failed to write validator count to storage")?;
                    "added validator"
                };
                state
                    .put_validator(&self.action)
                    .wrap_err("failed to write validator info to storage")?;
                debug!(
                    address = %self.action.verification_key.display_address(),
                    power = self.action.power,
                    log_msg,
                );
            }
        }

        // Add validator update in nonverifiable state to be used in end_block.
        let mut validator_updates = state
            .get_block_validator_updates()
            .await
            .wrap_err("failed to read validator updates from storage")?;
        validator_updates.insert(self.action.clone());
        state
            .put_block_validator_updates(validator_updates)
            .wrap_err("failed to write validator updates to storage")?;
        Ok(())
    }

    pub(super) fn action(&self) -> &ValidatorUpdate {
        &self.action
    }
}

impl AssetTransfer for CheckedValidatorUpdate {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        None
    }
}

struct Metadata {
    current_validator_count: u64,
    validator_already_exists: bool,
}

pub(crate) async fn use_pre_aspen_validator_updates<S: StateRead>(state: &S) -> Result<bool> {
    let pre_aspen_upgrade = state
        .get_upgrade_change_info(&Aspen::NAME, &ValidatorUpdateActionChange::NAME)
        .await
        .wrap_err(
            "failed to read upgrade change info for validator update action change from storage",
        )?
        .is_none();
    Ok(pre_aspen_upgrade)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::large_futures, reason = "test-only code")]

    use astria_core::{
        crypto::VerificationKey,
        protocol::transaction::v1::action::*,
    };

    use super::*;
    use crate::{
        checked_actions::CheckedSudoAddressChange,
        test_utils::{
            assert_error_contains,
            astria_address,
            Fixture,
            ALICE,
            BOB,
            SUDO_ADDRESS_BYTES,
        },
    };

    fn dummy_validator_update(power: u32, verification_key_bytes: [u8; 32]) -> ValidatorUpdate {
        ValidatorUpdate {
            power,
            verification_key: VerificationKey::try_from(verification_key_bytes).unwrap(),
            name: "validator name".parse().unwrap(),
        }
    }

    async fn new_fixture(use_pre_aspen: bool) -> Fixture {
        if use_pre_aspen {
            let mut fixture = Fixture::uninitialized(None).await;
            fixture.chain_initializer().init().await;
            assert!(use_pre_aspen_validator_updates(fixture.state())
                .await
                .unwrap());
            fixture
        } else {
            let fixture = Fixture::default_initialized().await;
            assert!(!use_pre_aspen_validator_updates(fixture.state())
                .await
                .unwrap());
            fixture
        }
    }

    async fn should_fail_construction_if_signer_is_not_sudo_address(use_pre_aspen: bool) {
        let fixture = new_fixture(use_pre_aspen).await;

        let tx_signer = [2_u8; ADDRESS_LEN];
        assert_ne!(*SUDO_ADDRESS_BYTES, tx_signer);

        let action = dummy_validator_update(100, [0; 32]);
        let err = fixture
            .new_checked_action(action, tx_signer)
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to update validator set",
        );
    }

    async fn should_fail_construction_if_removing_non_existent_validator(use_pre_aspen: bool) {
        let fixture = new_fixture(use_pre_aspen).await;

        let action = dummy_validator_update(0, [10; 32]);

        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();
        assert_error_contains(&err, "cannot remove a non-existing validator");
    }

    async fn should_fail_construction_if_removing_only_validator(use_pre_aspen: bool) {
        let mut fixture = Fixture::uninitialized(None).await;
        fixture
            .chain_initializer()
            .with_genesis_validators(Some((ALICE.verification_key(), 100)))
            .init()
            .await;
        if use_pre_aspen {
            assert!(use_pre_aspen_validator_updates(fixture.state())
                .await
                .unwrap());
        } else {
            let _ = fixture.run_until_blackburn_applied().await;
            assert!(!use_pre_aspen_validator_updates(fixture.state())
                .await
                .unwrap());
        }

        let action = dummy_validator_update(0, ALICE.verification_key().to_bytes());

        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();
        assert_error_contains(&err, "cannot remove the only validator");
    }

    async fn should_fail_execution_if_signer_is_not_sudo_address(use_pre_aspen: bool) {
        let mut fixture = new_fixture(use_pre_aspen).await;

        // Construct the checked action while the sudo address is still the tx signer so
        // construction succeeds.
        let action = dummy_validator_update(99, ALICE.verification_key().to_bytes());
        let checked_action: CheckedValidatorUpdate = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Change the sudo address to something other than the tx signer.
        let sudo_address_change = SudoAddressChange {
            new_address: astria_address(&[2; ADDRESS_LEN]),
        };
        let checked_sudo_address_change: CheckedSudoAddressChange = fixture
            .new_checked_action(sudo_address_change, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_sudo_address_change
            .execute(fixture.state_mut())
            .await
            .unwrap();
        let new_sudo_address = fixture.state().get_sudo_address().await.unwrap();
        assert_ne!(*SUDO_ADDRESS_BYTES, new_sudo_address);

        // Try to execute the checked action now - should fail due to signer no longer being
        // authorized.
        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to update validator set",
        );
    }

    async fn should_fail_execution_if_removing_non_existent_validator(use_pre_aspen: bool) {
        let mut fixture = new_fixture(use_pre_aspen).await;

        // Construct two checked actions to remove the same validator while it is still a validator
        // so construction succeeds.
        let action = dummy_validator_update(0, ALICE.verification_key().to_bytes());
        let checked_action_1: CheckedValidatorUpdate = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        let checked_action_2: CheckedValidatorUpdate = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Execute the first checked validator update.  We need to also run `end_block` in the
        // `AuthorityComponent` to actually have the validator set updated.
        checked_action_1.execute(fixture.state_mut()).await.unwrap();
        fixture.app.authority_component_end_block().await;

        // Try to execute the second checked action now - should fail due to validator no longer
        // being in the set.
        let err = checked_action_2
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(&err, "cannot remove a non-existing validator");
    }

    async fn should_fail_execution_if_removing_only_validator(use_pre_aspen: bool) {
        let mut fixture = Fixture::uninitialized(None).await;
        fixture
            .chain_initializer()
            .with_genesis_validators([
                (ALICE.verification_key(), 100),
                (BOB.verification_key(), 100),
            ])
            .init()
            .await;
        if use_pre_aspen {
            assert!(use_pre_aspen_validator_updates(fixture.state())
                .await
                .unwrap());
        } else {
            let _ = fixture.run_until_blackburn_applied().await;
            assert!(!use_pre_aspen_validator_updates(fixture.state())
                .await
                .unwrap());
        }

        // Construct two checked actions to remove the only two validators while they are still
        // validators so construction succeeds.
        let action_1 = dummy_validator_update(0, ALICE.verification_key().to_bytes());
        let checked_action_1: CheckedValidatorUpdate = fixture
            .new_checked_action(action_1, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        let action_2 = dummy_validator_update(0, BOB.verification_key().to_bytes());
        let checked_action_2: CheckedValidatorUpdate = fixture
            .new_checked_action(action_2, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Execute the first checked validator update.  We need to also run `end_block` in the
        // `AuthorityComponent` to actually have the validator set updated.
        checked_action_1.execute(fixture.state_mut()).await.unwrap();
        fixture.app.authority_component_end_block().await;

        // Try to execute the second checked action now - should fail due to validator being the
        // only validator in the set.
        let err = checked_action_2
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(&err, "cannot remove the only validator");
    }

    async fn should_execute(use_pre_aspen: bool) {
        let mut fixture = new_fixture(use_pre_aspen).await;

        let action = dummy_validator_update(99, ALICE.verification_key().to_bytes());

        let checked_action: CheckedValidatorUpdate = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_action.execute(fixture.state_mut()).await.unwrap();

        let validator_updates = fixture.state().get_block_validator_updates().await.unwrap();
        assert_eq!(validator_updates.len(), 1);
        let retrieved_update = validator_updates.get(&action.verification_key).unwrap();
        assert_eq!(retrieved_update.verification_key, action.verification_key);
        assert_eq!(retrieved_update.power, action.power);

        // The validator is updated as part of executing the action for post-Aspen only. (For
        // pre-Aspen, it's updated during `end_block`).
        if !use_pre_aspen {
            let retrieved_validator = fixture
                .state()
                .get_validator(&*crate::test_utils::ALICE_ADDRESS_BYTES)
                .await
                .expect("should get validator")
                .expect("validator should not be None");
            assert_eq!(retrieved_validator, action);
        }
    }

    mod pre_aspen {
        #[tokio::test]
        async fn should_fail_construction_if_signer_is_not_sudo_address() {
            super::should_fail_construction_if_signer_is_not_sudo_address(true).await;
        }

        #[tokio::test]
        async fn should_fail_construction_if_removing_non_existent_validator() {
            super::should_fail_construction_if_removing_non_existent_validator(true).await;
        }

        #[tokio::test]
        async fn should_fail_construction_if_removing_only_validator() {
            super::should_fail_construction_if_removing_only_validator(true).await;
        }

        #[tokio::test]
        async fn should_fail_execution_if_signer_is_not_sudo_address() {
            super::should_fail_execution_if_signer_is_not_sudo_address(true).await;
        }

        #[tokio::test]
        async fn should_fail_execution_if_removing_non_existent_validator() {
            super::should_fail_execution_if_removing_non_existent_validator(true).await;
        }

        #[tokio::test]
        async fn should_fail_execution_if_removing_only_validator() {
            super::should_fail_execution_if_removing_only_validator(true).await;
        }

        #[tokio::test]
        async fn should_execute() {
            super::should_execute(true).await;
        }
    }

    mod post_aspen {
        #[tokio::test]
        async fn should_fail_construction_if_signer_is_not_sudo_address() {
            super::should_fail_construction_if_signer_is_not_sudo_address(false).await;
        }

        #[tokio::test]
        async fn should_fail_construction_if_removing_non_existent_validator() {
            super::should_fail_construction_if_removing_non_existent_validator(false).await;
        }

        #[tokio::test]
        async fn should_fail_construction_if_removing_only_validator() {
            super::should_fail_construction_if_removing_only_validator(false).await;
        }

        #[tokio::test]
        async fn should_fail_execution_if_signer_is_not_sudo_address() {
            super::should_fail_execution_if_signer_is_not_sudo_address(false).await;
        }

        #[tokio::test]
        async fn should_fail_execution_if_removing_non_existent_validator() {
            super::should_fail_execution_if_removing_non_existent_validator(false).await;
        }

        #[tokio::test]
        async fn should_fail_execution_if_removing_only_validator() {
            super::should_fail_execution_if_removing_only_validator(false).await;
        }

        #[tokio::test]
        async fn should_execute() {
            super::should_execute(false).await;
        }
    }
}
