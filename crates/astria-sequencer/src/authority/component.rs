use std::sync::Arc;

use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1::action::ValidatorUpdate,
};
use astria_eyre::eyre::{
    OptionExt as _,
    Result,
    WrapErr as _,
};
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};
use tracing::{
    debug,
    info,
    instrument,
    Level,
};

use super::{
    StateReadExt,
    StateWriteExt,
    ValidatorSet,
};
use crate::{
    accounts::AddressBytes,
    action_handler::impls::validator_update::use_pre_aspen_validator_updates,
    component::Component,
};

#[derive(Default)]
pub(crate) struct AuthorityComponent;

#[derive(Debug)]
pub(crate) struct AuthorityComponentAppState {
    pub(crate) authority_sudo_address: Address,
    pub(crate) genesis_validators: Vec<ValidatorUpdate>,
}

impl AuthorityComponent {
    #[instrument(skip_all, err(level = Level::WARN))]
    pub(crate) async fn handle_aspen_upgrade<S: StateWriteExt>(state: &mut S) -> Result<()> {
        info!("performing Aspen upgrade validator set changes");
        let validator_set = state
            .pre_aspen_get_validator_set()
            .await
            .wrap_err("failed to get validator set")?;
        state
            .aspen_upgrade_remove_validator_set()
            .wrap_err("failed to remove validator set")?;
        state
            .put_validator_count(validator_set.len() as u64)
            .wrap_err("failed to put validator count")?;
        for (address, validator) in validator_set.0 {
            state.put_validator(&validator).wrap_err(format!(
                "failed to put validator info for validator {}",
                address.display_address()
            ))?;
            debug!(
                "validator {} successfully migrated from pre-aspen validator collection to \
                 post-aspen individual validator storage",
                address.display_address()
            );
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Component for AuthorityComponent {
    type AppState = AuthorityComponentAppState;

    #[instrument(name = "AuthorityComponent::init_chain", skip_all, err)]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        // set sudo key and initial validator set
        state
            .put_sudo_address(app_state.authority_sudo_address)
            .wrap_err("failed to set sudo key")?;
        let genesis_validators = app_state.genesis_validators.clone();
        state
            .pre_aspen_put_validator_set(ValidatorSet::new_from_updates(genesis_validators))
            .wrap_err("failed to set validator set")?;
        Ok(())
    }

    #[instrument(name = "AuthorityComponent::begin_block", skip_all, err(level = Level::WARN))]
    async fn begin_block<S: StateWriteExt + 'static>(
        state: &mut Arc<S>,
        begin_block: &BeginBlock,
    ) -> Result<()> {
        let state = Arc::get_mut(state)
            .ok_or_eyre("must only have one reference to the state; this is a bug")?;

        if use_pre_aspen_validator_updates(state)
            .await
            .wrap_err("failed to determine upgrade status")?
        {
            let mut current_set = state
                .pre_aspen_get_validator_set()
                .await
                .wrap_err("failed getting validator set")?;
            for misbehaviour in &begin_block.byzantine_validators {
                current_set.remove(&misbehaviour.validator.address);
            }
            state
                .pre_aspen_put_validator_set(current_set)
                .wrap_err("failed putting validator set")?;
        } else {
            for misbehaviour in &begin_block.byzantine_validators {
                // Only update the count if validator is in state
                if state
                    .remove_validator(&misbehaviour.validator.address)
                    .await
                    .wrap_err("failed to remove validator")?
                {
                    let validator_count = state
                        .get_validator_count()
                        .await
                        .wrap_err("failed to get validator count")?;
                    state
                        .put_validator_count(validator_count.saturating_sub(1))
                        .wrap_err("failed to put validator count")?;
                }
            }
        }
        Ok(())
    }

    #[instrument(name = "AuthorityComponent::end_block", skip_all, err(level = Level::WARN))]
    async fn end_block<S: StateWriteExt + StateReadExt + 'static>(
        state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        let state = Arc::get_mut(state)
            .ok_or_eyre("must only have one reference to the state; this is a bug")?;

        // If post Aspen upgrade, we don't need to do anything here since updates are made when the
        // `ValidatorUpdate` action is executed
        if use_pre_aspen_validator_updates(state)
            .await
            .wrap_err("failed to determine upgrade status")?
        {
            let validator_updates = state
                .get_block_validator_updates()
                .await
                .wrap_err("failed getting validator updates")?;

            let mut current_set = state
                .pre_aspen_get_validator_set()
                .await
                .wrap_err("failed getting validator set")?;
            current_set.apply_updates(validator_updates);

            state
                .pre_aspen_put_validator_set(current_set)
                .wrap_err("failed putting validator set")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        crypto::VerificationKey,
        protocol::transaction::v1::action::ValidatorName,
        upgrades::{
            test_utils::UpgradesBuilder,
            v1::Aspen,
        },
    };
    use cnidarium::StateDelta;
    use futures::TryStreamExt as _;
    use tendermint::{
        abci::types::{
            CommitInfo,
            Misbehavior,
            MisbehaviorKind,
            Validator,
        },
        account,
        block::{
            header::Version,
            Header,
            Height,
        },
        AppHash,
        Hash,
        Time,
    };

    use super::*;
    use crate::{
        authority::StateWriteExt as _,
        upgrades::StateWriteExt,
    };

    fn test_validator_set() -> ValidatorSet {
        ValidatorSet::new_from_updates(vec![
            ValidatorUpdate {
                name: ValidatorName::empty(),
                power: 10,
                verification_key: VerificationKey::try_from([0; 32]).unwrap(),
            },
            ValidatorUpdate {
                name: ValidatorName::empty(),
                power: 20,
                verification_key: VerificationKey::try_from([1; 32]).unwrap(),
            },
            ValidatorUpdate {
                name: ValidatorName::empty(),
                power: 30,
                verification_key: VerificationKey::try_from([3; 32]).unwrap(),
            },
        ])
    }

    fn test_begin_block() -> BeginBlock {
        BeginBlock {
            byzantine_validators: vec![],
            hash: Hash::default(),
            header: Header {
                version: Version {
                    block: 0,
                    app: 0,
                },
                chain_id: "chain_id".try_into().unwrap(),
                height: Height::default(),
                time: Time::now(),
                last_block_id: None,
                last_commit_hash: None,
                data_hash: None,
                validators_hash: Hash::default(),
                next_validators_hash: Hash::default(),
                consensus_hash: Hash::default(),
                app_hash: AppHash::default(),
                last_results_hash: None,
                evidence_hash: None,
                proposer_address: account::Id::new([0; 20]),
            },
            last_commit_info: CommitInfo {
                round: 0u16.into(),
                votes: vec![],
            },
        }
    }

    #[tokio::test]
    async fn handle_aspen_updrade_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // Create a validator set with 3 validators
        let validator_set = test_validator_set();

        state
            .pre_aspen_put_validator_set(validator_set.clone())
            .unwrap();

        // Check that validator set is stored correctly
        assert_eq!(
            state.pre_aspen_get_validator_set().await.unwrap(),
            validator_set
        );

        // Check that the individual validators are not stored yet
        for (address, _) in validator_set.0.clone() {
            assert!(state.get_validator(&address).await.unwrap().is_none());
        }
        // Check that the validator count is not set yet
        assert_eq!(
            state.get_validator_count().await.unwrap_err().to_string(),
            "validator count not found in state"
        );

        // Perform the Aspen upgrade
        AuthorityComponent::handle_aspen_upgrade(&mut state)
            .await
            .unwrap();

        // Check that the validator set is removed during upgrade
        assert_eq!(
            state
                .pre_aspen_get_validator_set()
                .await
                .unwrap_err()
                .to_string(),
            "validator set not found"
        );

        // Check validator count
        let validator_count = state.get_validator_count().await.unwrap();
        assert_eq!(validator_count, validator_set.len() as u64);

        // Check that the individual validators are stored correctly
        for (address, expected_validator) in validator_set.0 {
            let actual_validator = state.get_validator(&address).await.unwrap().unwrap();
            assert_eq!(actual_validator.name, expected_validator.name);
        }
    }

    #[tokio::test]
    async fn pre_aspen_begin_block_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let mut validator_set = test_validator_set();

        state
            .pre_aspen_put_validator_set(validator_set.clone())
            .unwrap();

        // Check that validator set is stored correctly
        assert_eq!(
            state.pre_aspen_get_validator_set().await.unwrap(),
            validator_set
        );

        // Create a BeginBlock request with a misbehaving validator
        let mut begin_block = test_begin_block();
        begin_block.byzantine_validators = vec![Misbehavior {
            validator: Validator {
                address: validator_set.0.pop_first().unwrap().0,
                power: 10u32.into(),
            },
            kind: MisbehaviorKind::Unknown,
            height: Height::default(),
            time: Time::now(),
            total_voting_power: 60u32.into(),
        }];

        let mut state = Arc::new(state);
        AuthorityComponent::begin_block(&mut state, &begin_block)
            .await
            .unwrap();

        // Check that the validator set is updated correctly
        let updated_validator_set = state.pre_aspen_get_validator_set().await.unwrap();
        assert_eq!(updated_validator_set.0.len(), 2);
        assert_eq!(updated_validator_set, validator_set);
    }

    #[tokio::test]
    async fn post_aspen_begin_block_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

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

        let mut validator_set = test_validator_set();

        for (index, (_, validator)) in validator_set.0.iter_mut().enumerate() {
            validator.name = format!("validator_{index}").parse().unwrap();
            state.put_validator(validator).unwrap();
        }

        state
            .put_validator_count(validator_set.0.len() as u64)
            .unwrap();

        // Check that validators are stored correctly
        assert_eq!(
            ValidatorSet::new_from_updates(state.get_validators().try_collect().await.unwrap()),
            validator_set
        );

        // Create a BeginBlock request with a misbehaving validator
        let mut begin_block = test_begin_block();
        begin_block.byzantine_validators = vec![Misbehavior {
            validator: Validator {
                address: validator_set.0.pop_first().unwrap().0,
                power: 10u32.into(),
            },
            kind: MisbehaviorKind::Unknown,
            height: Height::default(),
            time: Time::now(),
            total_voting_power: 60u32.into(),
        }];

        let mut state = Arc::new(state);
        AuthorityComponent::begin_block(&mut state, &begin_block)
            .await
            .unwrap();

        // Check that the validator set is updated correctly
        let updated_validator_set = state
            .get_validators()
            .try_collect::<Vec<ValidatorUpdate>>()
            .await
            .unwrap();
        assert_eq!(updated_validator_set.len(), 2);
        assert_eq!(
            ValidatorSet::new_from_updates(updated_validator_set),
            validator_set
        );
    }

    #[tokio::test]
    async fn pre_aspen_end_block_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let validator_set = test_validator_set();

        state
            .put_block_validator_updates(validator_set.clone())
            .unwrap();
        state
            .pre_aspen_put_validator_set(ValidatorSet::new_from_updates(vec![]))
            .unwrap();

        assert_eq!(
            state.pre_aspen_get_validator_set().await.unwrap(),
            ValidatorSet::new_from_updates(vec![])
        );

        let mut state = Arc::new(state);
        AuthorityComponent::end_block(
            &mut state,
            &EndBlock {
                height: 1.into(),
            },
        )
        .await
        .unwrap();

        // Check that the validator set is updated correctly
        let updated_validator_set = state.pre_aspen_get_validator_set().await.unwrap();
        assert_eq!(updated_validator_set, validator_set);
    }

    #[tokio::test]
    async fn post_aspen_end_block_does_nothing() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

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

        let validator_set = test_validator_set();
        state.put_block_validator_updates(validator_set).unwrap();

        let mut state = Arc::new(state);
        AuthorityComponent::end_block(
            &mut state,
            &EndBlock {
                height: 1.into(),
            },
        )
        .await
        .unwrap();

        let unchanged_validators = state
            .get_validators()
            .try_collect::<Vec<ValidatorUpdate>>()
            .await
            .unwrap();
        assert_eq!(unchanged_validators, vec![]);
    }
}
