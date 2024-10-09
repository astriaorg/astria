use std::sync::Arc;

use astria_core::protocol::{
    genesis::v1alpha1::GenesisAppState,
    transaction::v1alpha1::action::{
        BridgeLockFeeComponents,
        BridgeSudoChangeFeeComponents,
        BridgeUnlockFeeComponents,
        Ics20WithdrawalFeeComponents,
        InitBridgeAccountFeeComponents,
        SequenceFeeComponents,
        TransferFeeComponents,
    },
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

    #[instrument(name = "FeesComponent::init_chain", skip_all)]
    async fn init_chain<S>(mut state: S, app_state: &Self::AppState) -> Result<()>
    where
        S: fees::StateWriteExt + fees::StateReadExt,
    {
        let transfer_fees = TransferFeeComponents {
            base_fee: app_state.genesis_fees().transfer_fees.base_fee,
            computed_cost_multiplier: app_state
                .genesis_fees()
                .transfer_fees
                .computed_cost_multiplier,
        };
        state
            .put_transfer_fees(transfer_fees)
            .wrap_err("failed to initiate transfer fee components")?;

        let sequence_fees = SequenceFeeComponents {
            base_fee: app_state.genesis_fees().sequence_fees.base_fee,
            computed_cost_multiplier: app_state
                .genesis_fees()
                .sequence_fees
                .computed_cost_multiplier,
        };
        state
            .put_sequence_fees(sequence_fees)
            .wrap_err("failed to initiate sequence action fee components")?;

        let ics20_withdrawal_fees = Ics20WithdrawalFeeComponents {
            base_fee: app_state.genesis_fees().ics20_withdrawal_fees.base_fee,
            computed_cost_multiplier: app_state
                .genesis_fees()
                .ics20_withdrawal_fees
                .computed_cost_multiplier,
        };
        state
            .put_ics20_withdrawal_fees(ics20_withdrawal_fees)
            .wrap_err("failed to initiate ics20 withdrawal fee components")?;

        let init_bridge_account_fees = InitBridgeAccountFeeComponents {
            base_fee: app_state.genesis_fees().init_bridge_account_fees.base_fee,
            computed_cost_multiplier: app_state
                .genesis_fees()
                .init_bridge_account_fees
                .computed_cost_multiplier,
        };
        state
            .put_init_bridge_account_fees(init_bridge_account_fees)
            .wrap_err("failed to initiate init bridge account fee components")?;

        let bridge_lock_fees = BridgeLockFeeComponents {
            base_fee: app_state.genesis_fees().bridge_lock_fees.base_fee,
            computed_cost_multiplier: app_state
                .genesis_fees()
                .bridge_lock_fees
                .computed_cost_multiplier,
        };
        state
            .put_bridge_lock_fees(bridge_lock_fees)
            .wrap_err("failed to initiate bridge lock fee components")?;

        let bridge_unlock_fees = BridgeUnlockFeeComponents {
            base_fee: app_state.genesis_fees().bridge_unlock_fees.base_fee,
            computed_cost_multiplier: app_state
                .genesis_fees()
                .bridge_unlock_fees
                .computed_cost_multiplier,
        };
        state
            .put_bridge_unlock_fees(bridge_unlock_fees)
            .wrap_err("failed to initiate bridge unlock fee components")?;

        let bridge_sudo_change_fees = BridgeSudoChangeFeeComponents {
            base_fee: app_state.genesis_fees().bridge_sudo_change_fees.base_fee,
            computed_cost_multiplier: app_state
                .genesis_fees()
                .bridge_sudo_change_fees
                .computed_cost_multiplier,
        };
        state
            .put_bridge_sudo_change_fees(bridge_sudo_change_fees)
            .wrap_err("failed to initiate bridge sudo change fee components")?;

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
