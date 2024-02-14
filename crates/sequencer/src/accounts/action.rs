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
    state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::action_handler::ActionHandler,
};

/// Fee charged for a `Transfer` action.
pub(crate) const TRANSFER_FEE: u128 = 12;

#[async_trait::async_trait]
impl ActionHandler for TransferAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        ensure!(
            state.is_allowed_fee_asset(self.fee_asset_id).await?,
            "invalid fee asset",
        );

        let transfer_asset_id = self.asset_id;

        let from_fee_balance = state
            .get_account_balance(from, self.fee_asset_id)
            .await
            .context("failed getting `from` account balance for fee payment")?;

        // if fee asset is same as transfer asset, ensure accounts has enough funds
        // to cover both the fee and the amount transferred
        if self.fee_asset_id == transfer_asset_id {
            let payment_amount = self
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
                from_transfer_balance >= self.amount,
                "insufficient funds for transfer"
            );
        }

        Ok(())
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

        let from_balance = state
            .get_account_balance(from, transfer_asset_id)
            .await
            .context("failed getting `from` account balance")?;
        let to_balance = state
            .get_account_balance(self.to, transfer_asset_id)
            .await
            .context("failed getting `to` account balance")?;

        // if fee payment asset is same asset as transfer asset, deduct fee
        // from same balance as asset transferred
        if transfer_asset_id == self.fee_asset_id {
            // check_stateful should have already checked this arithmetic
            let payment_amount = self
                .amount
                .checked_add(TRANSFER_FEE)
                .expect("transfer amount plus fee should not overflow");

            state
                .put_account_balance(
                    from,
                    transfer_asset_id,
                    from_balance
                        .checked_sub(payment_amount)
                        .ok_or(anyhow!("insufficient funds for transfer and fee payment"))?,
                )
                .context("failed updating `from` account balance")?;
            state
                .put_account_balance(
                    self.to,
                    transfer_asset_id,
                    to_balance
                        .checked_add(self.amount)
                        .ok_or(anyhow!("recipient balance overflowed"))?,
                )
                .context("failed updating `to` account balance")?;
        } else {
            // otherwise, just transfer the transfer asset and deduct fee from fee asset balance
            // later
            state
                .put_account_balance(
                    from,
                    transfer_asset_id,
                    from_balance
                        .checked_sub(self.amount)
                        .ok_or(anyhow!("insufficient funds for transfer"))?,
                )
                .context("failed updating `from` account balance")?;
            state
                .put_account_balance(
                    self.to,
                    transfer_asset_id,
                    to_balance
                        .checked_add(self.amount)
                        .ok_or(anyhow!("recipient balance overflowed"))?,
                )
                .context("failed updating `to` account balance")?;

            // deduct fee from fee asset balance
            let from_fee_balance = state
                .get_account_balance(from, self.fee_asset_id)
                .await
                .context("failed getting `from` account balance for fee payment")?;
            state
                .put_account_balance(
                    from,
                    self.fee_asset_id,
                    from_fee_balance
                        .checked_sub(TRANSFER_FEE)
                        .ok_or(anyhow!("insufficient funds for fee payment"))?,
                )
                .context("failed updating `from` account balance for fee payment")?;
        }

        Ok(())
    }
}
