use astria_core::protocol::transaction::v1::action;
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        ensure,
        Result,
        WrapErr as _,
    },
};
use cnidarium::StateWrite;
use ibc_types::lightclients::tendermint::client_state::ClientState;
use penumbra_ibc::component::{
    ClientStateReadExt as _,
    ClientStateWriteExt as _,
    ClientStatus,
    ConsensusStateWriteExt as _,
};

use crate::{
    action_handler::ActionHandler,
    app::StateReadExt as _,
    authority::StateReadExt as _,
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for action::RecoverIbcClient {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        ensure!(
            from == state
                .get_sudo_address()
                .await
                .wrap_err("failed to get sudo address")?,
            "only the sudo address can recover a client",
        );

        let timestamp = state
            .get_block_timestamp()
            .await
            .wrap_err("failed to get timestamp")?;
        let client_status = state.get_client_status(&self.client_id, timestamp).await;

        // the spec requires the client to be either frozen or expired, but there is another
        // variant other than active, which is `ClientStatus::Unknown`.
        //
        // since unknown is only returned if there's an error calculating the status,
        // we can assume it's safe to only check for not-active as an error calculating
        // the status would cause various other errors.
        ensure!(
            client_status != ClientStatus::Active,
            "cannot recover an active client",
        );

        let replacement_client_status = state
            .get_client_status(&self.replacement_client_id, timestamp)
            .await;

        ensure!(
            replacement_client_status == ClientStatus::Active,
            "substitute client must be active: status is {}",
            replacement_client_status,
        );

        let mut client_state = state
            .get_client_state(&self.client_id)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("subject client state not found")?;
        let replacement_client_state = state
            .get_client_state(&self.replacement_client_id)
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

        let substitute_consensus_state = state
            .get_verified_consensus_state(
                &replacement_client_state.latest_height(),
                &self.replacement_client_id,
            )
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get verified consensus state")?;
        state
            .put_verified_consensus_state::<crate::ibc::host_interface::AstriaHost>(
                replacement_client_state.latest_height(),
                self.client_id.clone(),
                substitute_consensus_state,
            )
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to put verified consensus state")?;

        client_state.latest_height = replacement_client_state.latest_height;
        client_state.trusting_period = replacement_client_state.trusting_period;
        client_state.chain_id = replacement_client_state.chain_id;
        state.put_client(&self.client_id, client_state);
        Ok(())
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
