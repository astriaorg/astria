use std::sync::Arc;

use astria_core::protocol::genesis::v1::{
    GenesisAppState,
    GenesisFees,
};
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

        let GenesisFees {
            rollup_data_submission,
            transfer,
            ics20_withdrawal,
            init_bridge_account,
            bridge_lock,
            bridge_unlock,
            bridge_transfer,
            bridge_sudo_change,
            ibc_relay,
            validator_update,
            fee_asset_change,
            fee_change,
            ibc_relayer_change,
            sudo_address_change,
            ibc_sudo_change,
            recover_ibc_client,
            currency_pairs_change,
            markets_change,
        } = app_state.fees().clone();

        if let Some(transfer_fees) = transfer {
            state
                .put_fees(transfer_fees)
                .wrap_err("failed to store transfer fee components")?;
        }

        if let Some(rollup_data_submission_fees) = rollup_data_submission {
            state
                .put_fees(rollup_data_submission_fees)
                .wrap_err("failed to store rollup data submission fee components")?;
        }

        if let Some(ics20_withdrawal_fees) = ics20_withdrawal {
            state
                .put_fees(ics20_withdrawal_fees)
                .wrap_err("failed to store ics20 withdrawal fee components")?;
        }

        if let Some(init_bridge_account_fees) = init_bridge_account {
            state
                .put_fees(init_bridge_account_fees)
                .wrap_err("failed to store init bridge account fee components")?;
        }

        if let Some(bridge_lock_fees) = bridge_lock {
            state
                .put_fees(bridge_lock_fees)
                .wrap_err("failed to store bridge lock fee components")?;
        }

        if let Some(bridge_unlock_fees) = bridge_unlock {
            state
                .put_fees(bridge_unlock_fees)
                .wrap_err("failed to store bridge unlock fee components")?;
        }

        if let Some(bridge_transfer_fees) = bridge_transfer {
            state
                .put_fees(bridge_transfer_fees)
                .wrap_err("failed to store bridge transfer fee components")?;
        }

        if let Some(bridge_sudo_change_fees) = bridge_sudo_change {
            state
                .put_fees(bridge_sudo_change_fees)
                .wrap_err("failed to store bridge sudo change fee components")?;
        }

        if let Some(ibc_relay_fees) = ibc_relay {
            state
                .put_fees(ibc_relay_fees)
                .wrap_err("failed to store ibc relay fee components")?;
        }

        if let Some(validator_update_fees) = validator_update {
            state
                .put_fees(validator_update_fees)
                .wrap_err("failed to store validator update fee components")?;
        }

        if let Some(fee_asset_change_fees) = fee_asset_change {
            state
                .put_fees(fee_asset_change_fees)
                .wrap_err("failed to store fee asset change fee components")?;
        }

        state
            .put_fees(fee_change)
            .wrap_err("failed to store fee change fee components")?;

        if let Some(ibc_relayer_change_fees) = ibc_relayer_change {
            state
                .put_fees(ibc_relayer_change_fees)
                .wrap_err("failed to store ibc relayer change fee components")?;
        }

        if let Some(sudo_address_change_fees) = sudo_address_change {
            state
                .put_fees(sudo_address_change_fees)
                .wrap_err("failed to store sudo address change fee components")?;
        }

        if let Some(ibc_sudo_change_fees) = ibc_sudo_change {
            state
                .put_fees(ibc_sudo_change_fees)
                .wrap_err("failed to store ibc sudo change fee components")?;
        }

        if let Some(recover_ibc_client_fees) = recover_ibc_client {
            state
                .put_fees(recover_ibc_client_fees)
                .wrap_err("failed to store recover ibc client fee components")?;
        }

        if let Some(currency_pairs_change_fees) = currency_pairs_change {
            state
                .put_fees(currency_pairs_change_fees)
                .wrap_err("failed to store currency pairs change fee components")?;
        }

        if let Some(markets_change_fees) = markets_change {
            state
                .put_fees(markets_change_fees)
                .wrap_err("failed to store markets change fee components")?;
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
