use std::sync::Arc;

use astria_core::protocol::genesis::v1::GenesisAppState;
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};
use tracing::instrument;

use crate::{
    component::Component,
    fees,
};

#[derive(Default)]
pub(crate) struct FeesComponent;

#[async_trait::async_trait]
impl Component for FeesComponent {
    type AppState = GenesisAppState;

    #[instrument(name = "FeesComponent::init_chain", skip_all, err)]
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
                .put_fees(transfer_fees)
                .wrap_err("failed to store transfer fee components")?;
        }

        let rollup_data_submission_fees = app_state.fees().rollup_data_submission;
        if let Some(rollup_data_submission_fees) = rollup_data_submission_fees {
            state
                .put_fees(rollup_data_submission_fees)
                .wrap_err("failed to store rollup data submission fee components")?;
        }

        let ics20_withdrawal_fees = app_state.fees().ics20_withdrawal;
        if let Some(ics20_withdrawal_fees) = ics20_withdrawal_fees {
            state
                .put_fees(ics20_withdrawal_fees)
                .wrap_err("failed to store ics20 withdrawal fee components")?;
        }

        let init_bridge_account_fees = app_state.fees().init_bridge_account;
        if let Some(init_bridge_account_fees) = init_bridge_account_fees {
            state
                .put_fees(init_bridge_account_fees)
                .wrap_err("failed to store init bridge account fee components")?;
        }

        let bridge_lock_fees = app_state.fees().bridge_lock;
        if let Some(bridge_lock_fees) = bridge_lock_fees {
            state
                .put_fees(bridge_lock_fees)
                .wrap_err("failed to store bridge lock fee components")?;
        }

        let bridge_unlock_fees = app_state.fees().bridge_unlock;
        if let Some(bridge_unlock_fees) = bridge_unlock_fees {
            state
                .put_fees(bridge_unlock_fees)
                .wrap_err("failed to store bridge unlock fee components")?;
        }

        let bridge_sudo_change_fees = app_state.fees().bridge_sudo_change;
        if let Some(bridge_sudo_change_fees) = bridge_sudo_change_fees {
            state
                .put_fees(bridge_sudo_change_fees)
                .wrap_err("failed to store bridge sudo change fee components")?;
        }

        let ibc_relay_fees = app_state.fees().ibc_relay;
        if let Some(ibc_relay_fees) = ibc_relay_fees {
            state
                .put_fees(ibc_relay_fees)
                .wrap_err("failed to store ibc relay fee components")?;
        }

        let validator_update_fees = app_state.fees().validator_update;
        if let Some(validator_update_fees) = validator_update_fees {
            state
                .put_fees(validator_update_fees)
                .wrap_err("failed to store validator update fee components")?;
        }

        let fee_asset_change_fees = app_state.fees().fee_asset_change;
        if let Some(fee_asset_change_fees) = fee_asset_change_fees {
            state
                .put_fees(fee_asset_change_fees)
                .wrap_err("failed to store fee asset change fee components")?;
        }

        let fee_change_fees = app_state.fees().fee_change;
        state
            .put_fees(fee_change_fees)
            .wrap_err("failed to store fee change fee components")?;

        let ibc_relayer_change_fees = app_state.fees().ibc_relayer_change;
        if let Some(ibc_relayer_change_fees) = ibc_relayer_change_fees {
            state
                .put_fees(ibc_relayer_change_fees)
                .wrap_err("failed to store ibc relayer change fee components")?;
        }

        let sudo_address_change_fees = app_state.fees().sudo_address_change;
        if let Some(sudo_address_change_fees) = sudo_address_change_fees {
            state
                .put_fees(sudo_address_change_fees)
                .wrap_err("failed to store sudo address change fee components")?;
        }

        let ibc_sudo_change_fees = app_state.fees().ibc_sudo_change;
        if let Some(ibc_sudo_change_fees) = ibc_sudo_change_fees {
            state
                .put_fees(ibc_sudo_change_fees)
                .wrap_err("failed to store ibc sudo change fee components")?;
        }

        Ok(())
    }

    #[instrument(name = "FeesComponent::begin_block", skip_all)]
    async fn begin_block<S: fees::StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _begin_block: &BeginBlock,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "FeesComponent::end_block", skip_all)]
    async fn end_block<S: fees::StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _end_block: &EndBlock,
    ) -> Result<()> {
        Ok(())
    }
}
