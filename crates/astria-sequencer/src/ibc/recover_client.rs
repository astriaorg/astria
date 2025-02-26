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
use penumbra_ibc::component::{
    ClientStateReadExt as _,
    ClientStateWriteExt as _,
    // ConsensusStateWriteExt as _,
};

use crate::app::{
    ActionHandler,
    StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for action::RecoverClient {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        // TODO: ibc changes should be on ibc substore; other reads (height, timestamp)
        // are on normal store

        let timestamp = state
            .get_block_timestamp()
            .await
            .wrap_err("failed to get timestamp")?;
        let subject_client_status = state
            .get_client_status(&self.subject_client_id, timestamp)
            .await;

        // TODO: unfortunately need to string compare here as penumbra didn't export
        // penumbra_ibc::component::client::ClientStatus :/ (enum is pub but module isn't)
        ensure!(
            subject_client_status.to_string() != "Active",
            "cannot recover an active client",
        );

        let substitute_client_status = state
            .get_client_status(&self.substitute_client_id, timestamp)
            .await;

        ensure!(
            substitute_client_status.to_string() == "Active",
            "substitute client must be active: status is {}",
            substitute_client_status,
        );

        let mut subject_client_state = state
            .get_client_state(&self.subject_client_id)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("subject client state not found")?;
        let substitute_client_state = state
            .get_client_state(&self.substitute_client_id)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("substitute client state not found")?;

        ensure!(
            subject_client_state.latest_height() < substitute_client_state.latest_height(),
            "substitute client must have a higher height than that of subject client; subject \
             client height: {}, substitute client height: {}",
            subject_client_state.latest_height(),
            substitute_client_state.latest_height(),
        );

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
            .get_verified_consensus_state(&height, &self.substitute_client_id)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("substitute client consensus state not found")?;
        // state.put_verified_consensus_state(&height, &self.subject_client_id,
        // substitute_consensus_state).await.map_err(anyhow_to_eyre).wrap_err("failed to put
        // verified consensus state")?;

        subject_client_state.latest_height = substitute_client_state.latest_height;
        subject_client_state.chain_id = substitute_client_state.chain_id;
        subject_client_state.trusting_period = substitute_client_state.trusting_period;
        state.put_client(&self.subject_client_id, subject_client_state);
        Ok(())
    }
}
