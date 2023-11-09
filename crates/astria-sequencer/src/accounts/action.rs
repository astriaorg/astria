use anyhow::{
    ensure,
    Context,
    Result,
};
use proto::native::sequencer::{
    asset,
    v1alpha1::{
        Address,
        TransferAction,
    },
};
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    asset::NATIVE_ASSET,
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
        fee_asset: &asset::Id,
    ) -> Result<()> {
        let native_asset = *NATIVE_ASSET.get().expect("native asset must be set").id();
        let transfer_asset = self.asset.unwrap_or(native_asset);

        let curr_balance = state
            .get_account_balance(from, fee_asset)
            .await
            .context("failed getting `from` account balance for fee payment")?;

        // if fee asset is same as transfer asset, ensure accounts has enough funds
        // to cover both the fee and the amount transferred
        if fee_asset == &transfer_asset {
            ensure!(
                curr_balance >= self.amount + TRANSFER_FEE,
                "insufficient funds for transfer and fee payment"
            );
        } else {
            // otherwise, check the fee asset account has enough to cover the fees,
            // and the transfer asset account has enough to cover the transfer
            ensure!(
                curr_balance >= TRANSFER_FEE,
                "insufficient funds for fee payment"
            );

            let curr_balance = state.get_account_balance(from, &transfer_asset).await?;
            ensure!(
                curr_balance >= self.amount,
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
        fee_asset: &asset::Id,
    ) -> Result<()> {
        let native_asset = *NATIVE_ASSET.get().expect("native asset must be set").id();
        let transfer_asset = self.asset.unwrap_or(native_asset);

        let from_balance = state
            .get_account_balance(from, &transfer_asset)
            .await
            .context("failed getting `from` account balance")?;
        let to_balance = state
            .get_account_balance(self.to, &transfer_asset)
            .await
            .context("failed getting `to` account balance")?;

        // if fee payment asset is same asset as transfer asset, deduct fee
        // from same balance as asset transferred
        if &transfer_asset == fee_asset {
            state
                .put_account_balance(
                    from,
                    &transfer_asset,
                    from_balance - (self.amount + TRANSFER_FEE),
                )
                .context("failed updating `from` account balance")?;
            state
                .put_account_balance(self.to, &transfer_asset, to_balance + self.amount)
                .context("failed updating `to` account balance")?;
        } else {
            // otherwise, just transfer the transfer asset and deduct fee from fee asset balance
            // later
            state
                .put_account_balance(from, &transfer_asset, from_balance - self.amount)
                .context("failed updating `from` account balance")?;
            state
                .put_account_balance(self.to, &transfer_asset, to_balance + self.amount)
                .context("failed updating `to` account balance")?;

            // deduct fee from fee asset balance
            let from_fee_balance = state
                .get_account_balance(from, fee_asset)
                .await
                .context("failed getting `from` account balance for fee payment")?;
            state
                .put_account_balance(from, fee_asset, from_fee_balance - TRANSFER_FEE)
                .context("failed updating `from` account balance for fee payment")?;
        }

        Ok(())
    }
}
