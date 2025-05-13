use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::RecoverIbcClient,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        ensure,
        Result,
        WrapErr as _,
    },
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use ibc_types::lightclients::tendermint::client_state::ClientState;
use penumbra_ibc::component::{
    ClientStateReadExt as _,
    ClientStateWriteExt as _,
    ClientStatus,
    ConsensusStateWriteExt as _,
};
use tracing::{
    instrument,
    Level,
};

use super::{
    AssetTransfer,
    TransactionSignerAddressBytes,
};
use crate::{
    app::StateReadExt as _,
    authority::StateReadExt as _,
};

#[derive(Debug)]
pub(crate) struct CheckedRecoverIbcClient {
    action: RecoverIbcClient,
    tx_signer: TransactionSignerAddressBytes,
}

impl CheckedRecoverIbcClient {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn new<S: StateRead>(
        action: RecoverIbcClient,
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

    async fn do_run_mutable_checks<S: StateRead>(&self, state: S) -> Result<ClientStates> {
        // Ensure the tx signer is the current sudo address.
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to read sudo address from storage")?;
        ensure!(
            &sudo_address == self.tx_signer.as_bytes(),
            "transaction signer not authorized to recover ibc client"
        );

        let timestamp = state
            .get_block_timestamp()
            .await
            .wrap_err("failed to get block timestamp")?;
        let client_status = state
            .get_client_status(&self.action.client_id, timestamp)
            .await;

        // the spec requires the client to be either frozen or expired, but there is another
        // variant other than active, which is `ClientStatus::Unknown`.
        //
        // since unknown is only returned if there's an error calculating the status,
        // we can assume it's safe to only check for not-active as an error calculating
        // the status would cause various other errors.
        ensure!(
            client_status != ClientStatus::Active,
            "cannot recover an active client"
        );

        let replacement_client_status = state
            .get_client_status(&self.action.replacement_client_id, timestamp)
            .await;

        ensure!(
            replacement_client_status == ClientStatus::Active,
            "substitute client must be active: status is {}",
            replacement_client_status,
        );

        let client_state = state
            .get_client_state(&self.action.client_id)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("subject client state not found")?;
        let replacement_client_state = state
            .get_client_state(&self.action.replacement_client_id)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("substitute client state not found")?;

        ensure!(
            client_state.latest_height() < replacement_client_state.latest_height(),
            "substitute client must have a higher height than that of subject client; subject \
             client height: {}, substitute client height: {}",
            client_state.latest_height(),
            replacement_client_state.latest_height(),
        );

        ensure_required_client_state_fields_match(&client_state, &replacement_client_state)?;

        Ok(ClientStates {
            client_state,
            replacement_client_state,
        })
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let ClientStates {
            mut client_state,
            replacement_client_state,
        } = self.do_run_mutable_checks(&state).await?;

        let substitute_consensus_state = state
            .get_verified_consensus_state(
                &replacement_client_state.latest_height(),
                &self.action.replacement_client_id,
            )
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get verified consensus state")?;
        state
            .put_verified_consensus_state::<crate::ibc::host_interface::AstriaHost>(
                replacement_client_state.latest_height(),
                self.action.client_id.clone(),
                substitute_consensus_state,
            )
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to put verified consensus state")?;

        client_state.latest_height = replacement_client_state.latest_height;
        client_state.trusting_period = replacement_client_state.trusting_period;
        client_state.chain_id = replacement_client_state.chain_id;
        state.put_client(&self.action.client_id, client_state);

        Ok(())
    }

    pub(super) fn action(&self) -> &RecoverIbcClient {
        &self.action
    }
}

impl AssetTransfer for CheckedRecoverIbcClient {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        None
    }
}

// according to the ADR, all fields must match except for the latest height, trusting period,
// frozen height, and chain ID: https://ibc.cosmos.network/architecture/adr-026-ibc-client-recovery-mechanisms/
//
// this function checks that the required fields match, except for `allow_update`, which is
// deprecated.
fn ensure_required_client_state_fields_match(
    client_state: &ClientState,
    replacement_client_state: &ClientState,
) -> Result<()> {
    ensure!(
        client_state.trust_level == replacement_client_state.trust_level,
        "substitute client trust level must match subject client trust level; subject client \
         trust level: {:?}, substitute client trust level: {:?}",
        client_state.trust_level,
        replacement_client_state.trust_level,
    );

    ensure!(
        client_state.unbonding_period == replacement_client_state.unbonding_period,
        "substitute client unbonding period must match subject client unbonding period; subject \
         client unbonding period: {:?}, substitute client unbonding period: {:?}",
        client_state.unbonding_period,
        replacement_client_state.unbonding_period,
    );

    ensure!(
        client_state.max_clock_drift == replacement_client_state.max_clock_drift,
        "substitute client max clock drift must match subject client max clock drift; subject \
         client max clock drift: {:?}, substitute client max clock drift: {:?}",
        client_state.max_clock_drift,
        replacement_client_state.max_clock_drift,
    );

    ensure!(
        client_state.proof_specs == replacement_client_state.proof_specs,
        "substitute client proof specs must match subject client proof specs; subject client \
         proof specs: {:?}, substitute client proof specs: {:?}",
        client_state.proof_specs,
        replacement_client_state.proof_specs,
    );

    ensure!(
        client_state.upgrade_path == replacement_client_state.upgrade_path,
        "substitute client upgrade path must match subject client upgrade path; subject client \
         upgrade path: {:?}, substitute client upgrade path: {:?}",
        client_state.upgrade_path,
        replacement_client_state.upgrade_path,
    );

    Ok(())
}

struct ClientStates {
    client_state: ClientState,
    replacement_client_state: ClientState,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use astria_core::protocol::transaction::v1::action::*;
    use ibc_proto::ics23::ProofSpec;
    use ibc_types::{
        core::connection::ChainId,
        lightclients::tendermint::{
            client_state::ClientState,
            TrustThreshold,
        },
    };

    use super::*;
    use crate::{
        checked_actions::CheckedSudoAddressChange,
        test_utils::{
            assert_error_contains,
            astria_address,
            dummy_ibc_client_state,
            dummy_recover_ibc_client,
            Fixture,
            SUDO_ADDRESS_BYTES,
        },
    };

    #[tokio::test]
    async fn should_fail_construction_if_signer_is_not_sudo_address() {
        let fixture = Fixture::default_initialized().await;

        let tx_signer = [2_u8; ADDRESS_LEN];
        assert_ne!(*SUDO_ADDRESS_BYTES, tx_signer);

        let action = dummy_recover_ibc_client();
        let err = fixture
            .new_checked_action(action, tx_signer)
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to recover ibc client",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_block_timestamp_not_available() {
        let mut fixture = Fixture::uninitialized(None).await;
        fixture.chain_initializer().init().await;

        let action = dummy_recover_ibc_client();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();
        assert_error_contains(&err, "failed to get block timestamp");
    }

    #[tokio::test]
    async fn should_fail_construction_if_client_status_is_active() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_recover_ibc_client();
        fixture
            .init_active_ibc_client(&action.client_id, dummy_ibc_client_state(3))
            .await;

        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();
        assert_error_contains(&err, "cannot recover an active client");
    }

    #[tokio::test]
    async fn should_fail_construction_if_replacement_client_status_is_not_active() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_recover_ibc_client();
        fixture
            .init_expired_ibc_client(&action.client_id, dummy_ibc_client_state(3))
            .await;
        fixture
            .init_expired_ibc_client(&action.replacement_client_id, dummy_ibc_client_state(3))
            .await;

        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();
        assert_error_contains(&err, "substitute client must be active");
    }

    #[tokio::test]
    async fn should_fail_construction_if_replacement_client_height_not_less_than_client_height() {
        let check = |height: u64| async move {
            let mut fixture = Fixture::default_initialized().await;

            let action = dummy_recover_ibc_client();
            fixture
                .init_expired_ibc_client(&action.client_id, dummy_ibc_client_state(3))
                .await;
            fixture
                .init_active_ibc_client(
                    &action.replacement_client_id,
                    dummy_ibc_client_state(height),
                )
                .await;

            let err = fixture
                .new_checked_action(action, *SUDO_ADDRESS_BYTES)
                .await
                .unwrap_err();
            assert_error_contains(
                &err,
                "substitute client must have a higher height than that of subject client",
            );
        };

        check(2).await;
        check(3).await;
    }

    async fn should_fail_construction_if_mismatch_in_client_state_field(
        replacement_client_state: ClientState,
        expected_error_message: &str,
    ) {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_recover_ibc_client();
        fixture
            .init_expired_ibc_client(&action.client_id, dummy_ibc_client_state(3))
            .await;
        fixture
            .init_active_ibc_client(&action.replacement_client_id, replacement_client_state)
            .await;

        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();
        assert_error_contains(&err, expected_error_message);
    }

    #[tokio::test]
    async fn should_fail_construction_if_mismatch_in_client_state_trust_level() {
        let replacement_client_state = ClientState {
            trust_level: TrustThreshold::ONE_THIRD,
            ..dummy_ibc_client_state(4)
        };
        should_fail_construction_if_mismatch_in_client_state_field(
            replacement_client_state,
            "substitute client trust level must match subject client trust level",
        )
        .await;
    }

    #[tokio::test]
    async fn should_fail_construction_if_mismatch_in_client_state_unbonding_period() {
        let replacement_client_state = ClientState {
            unbonding_period: Duration::from_secs(10),
            ..dummy_ibc_client_state(4)
        };
        should_fail_construction_if_mismatch_in_client_state_field(
            replacement_client_state,
            "substitute client unbonding period must match subject client unbonding period",
        )
        .await;
    }

    #[tokio::test]
    async fn should_fail_construction_if_mismatch_in_client_state_max_clock_drift() {
        let replacement_client_state = ClientState {
            max_clock_drift: Duration::from_secs(10),
            ..dummy_ibc_client_state(4)
        };
        should_fail_construction_if_mismatch_in_client_state_field(
            replacement_client_state,
            "substitute client max clock drift must match subject client max clock drift",
        )
        .await;
    }

    #[tokio::test]
    async fn should_fail_construction_if_mismatch_in_client_state_proof_specs() {
        let proof_spec = ProofSpec {
            leaf_spec: None,
            inner_spec: None,
            max_depth: 1,
            min_depth: 0,
            prehash_key_before_comparison: false,
        };
        let replacement_client_state = ClientState {
            proof_specs: vec![proof_spec],
            ..dummy_ibc_client_state(4)
        };
        should_fail_construction_if_mismatch_in_client_state_field(
            replacement_client_state,
            "substitute client proof specs must match subject client proof specs",
        )
        .await;
    }

    #[tokio::test]
    async fn should_fail_construction_if_mismatch_in_client_state_upgrade_path() {
        let replacement_client_state = ClientState {
            upgrade_path: vec!["a".to_string()],
            ..dummy_ibc_client_state(4)
        };
        should_fail_construction_if_mismatch_in_client_state_field(
            replacement_client_state,
            "substitute client upgrade path must match subject client upgrade path",
        )
        .await;
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_is_not_sudo_address() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct the checked action while the sudo address is still the tx signer so
        // construction succeeds.
        let action = dummy_recover_ibc_client();
        fixture
            .init_expired_ibc_client(&action.client_id, dummy_ibc_client_state(3))
            .await;
        fixture
            .init_active_ibc_client(&action.replacement_client_id, dummy_ibc_client_state(4))
            .await;

        let checked_action: CheckedRecoverIbcClient = fixture
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
            "transaction signer not authorized to recover ibc client",
        );
    }

    #[tokio::test]
    async fn should_execute() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_recover_ibc_client();

        // Prepare a replacement with different values for height, trusting period and chain ID.
        let expired_client_state = dummy_ibc_client_state(3);
        let replacement_client_state = ClientState {
            trusting_period: Duration::from_secs(10),
            chain_id: ChainId::new("different".to_string(), 2),
            ..dummy_ibc_client_state(4)
        };
        assert_ne!(
            expired_client_state.latest_height,
            replacement_client_state.latest_height
        );
        assert_ne!(
            expired_client_state.trusting_period,
            replacement_client_state.trusting_period
        );
        assert_ne!(
            expired_client_state.chain_id,
            replacement_client_state.chain_id
        );
        fixture
            .init_expired_ibc_client(&action.client_id, expired_client_state)
            .await;
        fixture
            .init_active_ibc_client(
                &action.replacement_client_id,
                replacement_client_state.clone(),
            )
            .await;

        // Check the client status before execution is `Expired`.
        let client_id = action.client_id.clone();
        let block_time = fixture.state().get_block_timestamp().await.unwrap();
        let status_before = fixture
            .state()
            .get_client_status(&client_id, block_time)
            .await;
        assert_eq!(status_before, ClientStatus::Expired);

        // Execute the checked action.
        let checked_action: CheckedRecoverIbcClient = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_action.execute(fixture.state_mut()).await.unwrap();

        // The client state should now hold the replacement values.
        let stored_client_state = fixture.state().get_client_state(&client_id).await.unwrap();
        assert_eq!(
            stored_client_state.latest_height,
            replacement_client_state.latest_height
        );
        assert_eq!(
            stored_client_state.trusting_period,
            replacement_client_state.trusting_period
        );
        assert_eq!(
            stored_client_state.chain_id,
            replacement_client_state.chain_id
        );

        // The client status should now be `Active`.
        let status_after = fixture
            .state()
            .get_client_status(&client_id, block_time)
            .await;
        assert_eq!(status_after, ClientStatus::Active);
    }
}
