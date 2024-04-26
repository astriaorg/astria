use anyhow::{
    ensure,
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::{
        BridgeLockAction,
        TransferAction,
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use tracing::instrument;

use crate::{
    accounts::{
        action::transfer_check_stateful,
        state_ext::{
            StateReadExt as _,
            StateWriteExt as _,
        },
    },
    bridge::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    transaction::action_handler::ActionHandler,
};

pub(crate) const DEPOSIT_BYTE_LEN: u128 = std::mem::size_of::<Deposit>() as u128;

#[async_trait::async_trait]
impl ActionHandler for BridgeLockAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        let transfer_action = TransferAction {
            to: self.to,
            asset_id: self.asset_id,
            amount: self.amount,
            fee_asset_id: self.fee_asset_id,
        };

        // ensure the recipient is a bridge account.
        ensure!(
            state
                .get_bridge_account_rollup_id(&self.to)
                .await?
                .is_some(),
            "bridge lock must be sent to a bridge account",
        );

        let allowed_asset_id = state
            .get_bridge_account_asset_ids(&self.to)
            .await
            .context("failed to get bridge account asset ID")?;
        ensure!(
            allowed_asset_id == self.asset_id,
            "asset ID is not authorized for transfer to bridge account",
        );

        let from_balance = state
            .get_account_balance(from, self.fee_asset_id)
            .await
            .context("failed to get sender account balance")?;
        let transfer_fee = state
            .get_transfer_base_fee()
            .await
            .context("failed to get transfer base fee")?;

        let byte_cost_multiplier = state
            .get_bridge_lock_byte_cost_multiplier()
            .await
            .context("failed to get byte cost multiplier")?;
        let fee = byte_cost_multiplier * DEPOSIT_BYTE_LEN + transfer_fee;
        ensure!(from_balance >= fee, "insuffient funds for fee payment",);

        // this performs the same checks as a normal `TransferAction`,
        // but without the check that prevents transferring to a bridge account,
        // as we are explicitly transferring to a bridge account here.
        transfer_check_stateful(&transfer_action, state, from).await
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        let transfer_action = TransferAction {
            to: self.to,
            asset_id: self.asset_id,
            amount: self.amount,
            fee_asset_id: self.fee_asset_id,
        };

        transfer_action
            .execute(state, from)
            .await
            .context("failed to execute bridge lock action as transfer action")?;

        // the transfer fee is already deducted in `transfer_action.execute()`,
        // so we just deduct the bridge lock byte multiplier fee.
        let byte_cost_multiplier = state
            .get_bridge_lock_byte_cost_multiplier()
            .await
            .context("failed to get byte cost multiplier")?;
        let fee = byte_cost_multiplier * DEPOSIT_BYTE_LEN;

        state
            .decrease_balance(from, self.fee_asset_id, fee)
            .await
            .context("failed to deduct fee from account balance")?;

        let rollup_id = state
            .get_bridge_account_rollup_id(&self.to)
            .await?
            .expect("recipient must be a bridge account; this is a bug in check_stateful");

        let deposit = Deposit::new(
            self.to,
            rollup_id,
            self.amount,
            self.asset_id,
            self.destination_chain_address.clone(),
        );
        state
            .put_deposit_event(deposit)
            .await
            .context("failed to put deposit event into state")?;
        Ok(())
    }
}
