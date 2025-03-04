use astria_core::protocol::transaction::v1::action;
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        ensure,
        OptionExt as _,
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
        let client_to_replace_status = state
            .get_client_status(&self.client_id_to_replace, timestamp)
            .await;

        ensure!(
            client_to_replace_status != ClientStatus::Active,
            "cannot recover an active client",
        );

        let substitute_client_status = state
            .get_client_status(&self.substitute_client_id, timestamp)
            .await;

        ensure!(
            substitute_client_status == ClientStatus::Active,
            "substitute client must be active: status is {}",
            substitute_client_status,
        );

        let mut client_to_replace_state = state
            .get_client_state(&self.client_id_to_replace)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("subject client state not found")?;
        let substitute_client_state = state
            .get_client_state(&self.substitute_client_id)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("substitute client state not found")?;

        ensure!(
            client_to_replace_state.latest_height() < substitute_client_state.latest_height(),
            "substitute client must have a higher height than that of subject client; subject \
             client height: {}, substitute client height: {}",
            client_to_replace_state.latest_height(),
            substitute_client_state.latest_height(),
        );

        ensure_required_client_state_fields_match(
            &client_to_replace_state,
            &substitute_client_state,
        )?;

        let height = ibc_types::core::client::Height {
            revision_height: state
                .get_block_height()
                .await
                .wrap_err("failed to get block height")?,
            revision_number: state
                .get_revision_number()
                .await
                .wrap_err("failed to get revision number")?,
        };
        let substitute_consensus_state = state
            .prev_verified_consensus_state(&self.substitute_client_id, &height)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get substitute client consensus state")?
            .ok_or_eyre("substitute client consensus state not found")?;
        state
            .put_verified_consensus_state::<crate::ibc::host_interface::AstriaHost>(
                substitute_client_state.latest_height(),
                self.client_id_to_replace.clone(),
                substitute_consensus_state,
            )
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to put verified consensus state")?;

        client_to_replace_state.latest_height = substitute_client_state.latest_height;
        client_to_replace_state.trusting_period = substitute_client_state.trusting_period;
        state.put_client(&self.client_id_to_replace, client_to_replace_state);
        Ok(())
    }
}

fn ensure_required_client_state_fields_match(
    client_to_replace_state: &ClientState,
    substitute_client_state: &ClientState,
) -> Result<()> {
    ensure!(
        client_to_replace_state.trust_level == substitute_client_state.trust_level,
        "substitute client trust level must match subject client trust level; subject client \
         trust level: {:?}, substitute client trust level: {:?}",
        client_to_replace_state.trust_level,
        substitute_client_state.trust_level,
    );

    ensure!(
        client_to_replace_state.unbonding_period == substitute_client_state.unbonding_period,
        "substitute client unbonding period must match subject client unbonding period; subject \
         client unbonding period: {:?}, substitute client unbonding period: {:?}",
        client_to_replace_state.unbonding_period,
        substitute_client_state.unbonding_period,
    );

    ensure!(
        client_to_replace_state.max_clock_drift == substitute_client_state.max_clock_drift,
        "substitute client max clock drift must match subject client max clock drift; subject \
         client max clock drift: {:?}, substitute client max clock drift: {:?}",
        client_to_replace_state.max_clock_drift,
        substitute_client_state.max_clock_drift,
    );

    ensure!(
        client_to_replace_state.proof_specs == substitute_client_state.proof_specs,
        "substitute client proof specs must match subject client proof specs; subject client \
         proof specs: {:?}, substitute client proof specs: {:?}",
        client_to_replace_state.proof_specs,
        substitute_client_state.proof_specs,
    );

    ensure!(
        client_to_replace_state.upgrade_path == substitute_client_state.upgrade_path,
        "substitute client upgrade path must match subject client upgrade path; subject client \
         upgrade path: {:?}, substitute client upgrade path: {:?}",
        client_to_replace_state.upgrade_path,
        substitute_client_state.upgrade_path,
    );

    Ok(())
}
