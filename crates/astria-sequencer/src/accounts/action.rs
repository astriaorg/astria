use anyhow::{
    ensure,
    Context,
    Result,
};
use proto::native::sequencer::v1alpha1::{
    asset,
    Address,
    TransferAction,
};
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt,
        StateWriteExt,
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
        fee_asset_id: &asset::Id,
    ) -> Result<()> {
        let transfer_asset_id = self.asset_id;

        let from_fee_balance = state
            .get_account_balance(from, fee_asset_id)
            .await
            .context("failed getting `from` account balance for fee payment")?;

        // if fee asset is same as transfer asset, ensure accounts has enough funds
        // to cover both the fee and the amount transferred
        if fee_asset_id == &transfer_asset_id {
            ensure!(
                from_fee_balance >= self.amount + TRANSFER_FEE,
                "insufficient funds for transfer and fee payment"
            );
        } else {
            // otherwise, check the fee asset account has enough to cover the fees,
            // and the transfer asset account has enough to cover the transfer
            ensure!(
                from_fee_balance >= TRANSFER_FEE,
                "insufficient funds for fee payment"
            );

            let from_transfer_balance = state.get_account_balance(from, &transfer_asset_id).await?;
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
    async fn execute<S: StateWriteExt>(
        &self,
        state: &mut S,
        from: Address,
        fee_asset_id: &asset::Id,
    ) -> Result<()> {
        let transfer_asset_id = self.asset_id;

        let from_balance = state
            .get_account_balance(from, &transfer_asset_id)
            .await
            .context("failed getting `from` account balance")?;
        let to_balance = state
            .get_account_balance(self.to, &transfer_asset_id)
            .await
            .context("failed getting `to` account balance")?;

        // if fee payment asset is same asset as transfer asset, deduct fee
        // from same balance as asset transferred
        if &transfer_asset_id == fee_asset_id {
            state
                .put_account_balance(
                    from,
                    &transfer_asset_id,
                    from_balance - (self.amount + TRANSFER_FEE),
                )
                .context("failed updating `from` account balance")?;
            state
                .put_account_balance(self.to, &transfer_asset_id, to_balance + self.amount)
                .context("failed updating `to` account balance")?;
        } else {
            // otherwise, just transfer the transfer asset and deduct fee from fee asset balance
            // later
            state
                .put_account_balance(from, &transfer_asset_id, from_balance - self.amount)
                .context("failed updating `from` account balance")?;
            state
                .put_account_balance(self.to, &transfer_asset_id, to_balance + self.amount)
                .context("failed updating `to` account balance")?;

            // deduct fee from fee asset balance
            let from_fee_balance = state
                .get_account_balance(from, fee_asset_id)
                .await
                .context("failed getting `from` account balance for fee payment")?;
            state
                .put_account_balance(from, fee_asset_id, from_fee_balance - TRANSFER_FEE)
                .context("failed updating `from` account balance for fee payment")?;
        }

        Ok(())
    }
}
