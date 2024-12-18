use std::sync::Arc;

use astria_core::protocol::genesis::v1::GenesisAppState;
use astria_eyre::eyre::{
    OptionExt as _,
    Result,
    WrapErr as _,
};
use tracing::instrument;

use super::StateReadExt as _;
use crate::{
    accounts::StateWriteExt as _,
    authority::StateReadExt,
    component::{
        Component,
        PrepareStateInfo,
    },
    fees,
};

#[derive(Default)]
pub(crate) struct FeesComponent;

#[async_trait::async_trait]
impl Component for FeesComponent {
    type AppState = GenesisAppState;

    #[instrument(name = "FeesComponent::init_chain", skip_all)]
    async fn init_chain<S>(mut state: S, app_state: &Self::AppState) -> Result<()>
    where
        S: fees::StateWriteExt + fees::StateReadExt,
    {
        for fee_asset in app_state.allowed_fee_assets() {
            state
                .put_allowed_fee_asset(fee_asset)
                .wrap_err("failed to write allowed fee asset to state")?;
        }

        let transfer_fees = app_state.fees().transfer;
        if let Some(transfer_fees) = transfer_fees {
            state
                .put_transfer_fees(transfer_fees)
                .wrap_err("failed to store transfer fee components")?;
        }

        let rollup_data_submission_fees = app_state.fees().rollup_data_submission;
        if let Some(rollup_data_submission_fees) = rollup_data_submission_fees {
            state
                .put_rollup_data_submission_fees(rollup_data_submission_fees)
                .wrap_err("failed to store rollup data submission fee components")?;
        }

        let ics20_withdrawal_fees = app_state.fees().ics20_withdrawal;
        if let Some(ics20_withdrawal_fees) = ics20_withdrawal_fees {
            state
                .put_ics20_withdrawal_fees(ics20_withdrawal_fees)
                .wrap_err("failed to store ics20 withdrawal fee components")?;
        }

        let init_bridge_account_fees = app_state.fees().init_bridge_account;
        if let Some(init_bridge_account_fees) = init_bridge_account_fees {
            state
                .put_init_bridge_account_fees(init_bridge_account_fees)
                .wrap_err("failed to store init bridge account fee components")?;
        }

        let bridge_lock_fees = app_state.fees().bridge_lock;
        if let Some(bridge_lock_fees) = bridge_lock_fees {
            state
                .put_bridge_lock_fees(bridge_lock_fees)
                .wrap_err("failed to store bridge lock fee components")?;
        }

        let bridge_unlock_fees = app_state.fees().bridge_unlock;
        if let Some(bridge_unlock_fees) = bridge_unlock_fees {
            state
                .put_bridge_unlock_fees(bridge_unlock_fees)
                .wrap_err("failed to store bridge unlock fee components")?;
        }

        let bridge_sudo_change_fees = app_state.fees().bridge_sudo_change;
        if let Some(bridge_sudo_change_fees) = bridge_sudo_change_fees {
            state
                .put_bridge_sudo_change_fees(bridge_sudo_change_fees)
                .wrap_err("failed to store bridge sudo change fee components")?;
        }

        let ibc_relay_fees = app_state.fees().ibc_relay;
        if let Some(ibc_relay_fees) = ibc_relay_fees {
            state
                .put_ibc_relay_fees(ibc_relay_fees)
                .wrap_err("failed to store ibc relay fee components")?;
        }

        let validator_update_fees = app_state.fees().validator_update;
        if let Some(validator_update_fees) = validator_update_fees {
            state
                .put_validator_update_fees(validator_update_fees)
                .wrap_err("failed to store validator update fee components")?;
        }

        let fee_asset_change_fees = app_state.fees().fee_asset_change;
        if let Some(fee_asset_change_fees) = fee_asset_change_fees {
            state
                .put_fee_asset_change_fees(fee_asset_change_fees)
                .wrap_err("failed to store fee asset change fee components")?;
        }

        let fee_change_fees = app_state.fees().fee_change;
        state
            .put_fee_change_fees(fee_change_fees)
            .wrap_err("failed to store fee change fee components")?;

        let ibc_relayer_change_fees = app_state.fees().ibc_relayer_change;
        if let Some(ibc_relayer_change_fees) = ibc_relayer_change_fees {
            state
                .put_ibc_relayer_change_fees(ibc_relayer_change_fees)
                .wrap_err("failed to store ibc relayer change fee components")?;
        }

        let sudo_address_change_fees = app_state.fees().sudo_address_change;
        if let Some(sudo_address_change_fees) = sudo_address_change_fees {
            state
                .put_sudo_address_change_fees(sudo_address_change_fees)
                .wrap_err("failed to store sudo address change fee components")?;
        }

        let ibc_sudo_change_fees = app_state.fees().ibc_sudo_change;
        if let Some(ibc_sudo_change_fees) = ibc_sudo_change_fees {
            state
                .put_ibc_sudo_change_fees(ibc_sudo_change_fees)
                .wrap_err("failed to store ibc sudo change fee components")?;
        }

        Ok(())
    }

    #[instrument(name = "FeesComponent::prepare_state_for_tx_execution", skip_all)]
    async fn prepare_state_for_tx_execution<S: fees::StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _prepare_state_info: &PrepareStateInfo,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "FeesComponent::handle_post_tx_execution", skip_all)]
    async fn handle_post_tx_execution<S: fees::StateWriteExt + 'static>(
        state: &mut Arc<S>,
    ) -> Result<()> {
        // gather block fees and transfer them to sudo
        let fees = state.get_block_fees();
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address for fee payment")?;

        let state_tx = Arc::get_mut(state)
            .ok_or_eyre("must only have one reference to the state; this is a bug")?;
        for fee in fees {
            state_tx
                .increase_balance(&sudo_address, fee.asset(), fee.amount())
                .await
                .wrap_err("failed to increase fee recipient balance")?;
        }

        Ok(())
    }
}
