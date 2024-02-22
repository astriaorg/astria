use anyhow::{
    anyhow,
    ensure,
    Context,
    Result,
};
use astria_core::sequencer::v1alpha1::{
    transaction::action::TransferAction,
    Address,
};
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    bridge::state_ext::StateReadExt as _,
    state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::action_handler::ActionHandler,
};

/// Fee charged for a `Transfer` action.
pub(crate) const TRANSFER_FEE: u128 = 12;

pub(crate) async fn transfer_check_stateful<S: StateReadExt + 'static>(
    action: &TransferAction,
    state: &S,
    from: Address,
) -> Result<()> {
    ensure!(
        state.is_allowed_fee_asset(action.fee_asset_id).await?,
        "invalid fee asset",
    );

    let transfer_asset_id = action.asset_id;

    let from_fee_balance = state
        .get_account_balance(from, action.fee_asset_id)
        .await
        .context("failed getting `from` account balance for fee payment")?;

    // if fee asset is same as transfer asset, ensure accounts has enough funds
    // to cover both the fee and the amount transferred
    if action.fee_asset_id == transfer_asset_id {
        let payment_amount = action
            .amount
            .checked_add(TRANSFER_FEE)
            .ok_or(anyhow!("transfer amount plus fee overflowed"))?;

        ensure!(
            from_fee_balance >= payment_amount,
            "insufficient funds for transfer and fee payment"
        );
    } else {
        // otherwise, check the fee asset account has enough to cover the fees,
        // and the transfer asset account has enough to cover the transfer
        ensure!(
            from_fee_balance >= TRANSFER_FEE,
            "insufficient funds for fee payment"
        );

        let from_transfer_balance = state
            .get_account_balance(from, transfer_asset_id)
            .await
            .context("failed to get account balance in transfer check")?;
        ensure!(
            from_transfer_balance >= action.amount,
            "insufficient funds for transfer"
        );
    }

    Ok(())
}

#[async_trait::async_trait]
impl ActionHandler for TransferAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        // ensure the recipient is not a bridge account,
        // as the explicit `BridgeLockAction` should be used to transfer to a bridge account.
        ensure!(
            state
                .get_bridge_account_rollup_id(&self.to)
                .await?
                .is_none(),
            "cannot send transfer to bridge account",
        );

        transfer_check_stateful(self, state, from).await
    }

    #[instrument(
        skip_all,
        fields(
            to = self.to.to_string(),
            amount = self.amount,
        )
    )]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        state
            .get_and_increase_block_fees(self.fee_asset_id, TRANSFER_FEE)
            .await
            .context("failed to add to block fees")?;

        let transfer_asset_id = self.asset_id;

        // if fee payment asset is same asset as transfer asset, deduct fee
        // from same balance as asset transferred
        if transfer_asset_id == self.fee_asset_id {
            // check_stateful should have already checked this arithmetic
            let payment_amount = self
                .amount
                .checked_add(TRANSFER_FEE)
                .expect("transfer amount plus fee should not overflow");

            state
                .decrease_balance(from, transfer_asset_id, payment_amount)
                .await
                .context("failed decreasing `from` account balance")?;
            state
                .increase_balance(self.to, transfer_asset_id, self.amount)
                .await
                .context("failed increasing `to` account balance")?;
        } else {
            // otherwise, just transfer the transfer asset and deduct fee from fee asset balance
            // later
            state
                .decrease_balance(from, transfer_asset_id, self.amount)
                .await
                .context("failed decreasing `from` account balance")?;
            state
                .increase_balance(self.to, transfer_asset_id, self.amount)
                .await
                .context("failed increasing `to` account balance")?;

            // deduct fee from fee asset balance
            state
                .decrease_balance(from, self.fee_asset_id, TRANSFER_FEE)
                .await
                .context("failed decreasing `from` account balance for fee payment")?;
        }

        Ok(())
    }
}
