use anyhow::{
    ensure,
    Context,
    Result,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::TransferAction,
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

pub(crate) async fn transfer_check_stateful<S: StateReadExt + 'static>(
    action: &TransferAction,
    state: &S,
    from: Address,
) -> Result<()> {
    ensure!(
        state
            .is_allowed_fee_asset(&action.fee_asset)
            .await
            .context("failed to check allowed fee assets in state")?,
        "invalid fee asset",
    );

    let fee = state
        .get_transfer_base_fee()
        .await
        .context("failed to get transfer base fee")?;
    let transfer_asset = action.asset.clone();

    let from_fee_balance = state
        .get_account_balance(from, &action.fee_asset)
        .await
        .context("failed getting `from` account balance for fee payment")?;

    // if fee asset is same as transfer asset, ensure accounts has enough funds
    // to cover both the fee and the amount transferred
    if action.fee_asset.to_ibc_prefixed() == transfer_asset.to_ibc_prefixed() {
        let payment_amount = action
            .amount
            .checked_add(fee)
            .context("transfer amount plus fee overflowed")?;

        ensure!(
            from_fee_balance >= payment_amount,
            "insufficient funds for transfer and fee payment"
        );
    } else {
        // otherwise, check the fee asset account has enough to cover the fees,
        // and the transfer asset account has enough to cover the transfer
        ensure!(
            from_fee_balance >= fee,
            "insufficient funds for fee payment"
        );

        let from_transfer_balance = state
            .get_account_balance(from, transfer_asset)
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
    async fn check_stateless(&self) -> Result<()> {
        crate::address::ensure_base_prefix(&self.to).context("destination address is invalid")?;
        Ok(())
    }

    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        ensure!(
            state
                .get_bridge_account_rollup_id(&from)
                .await
                .context("failed to get bridge account rollup id")?
                .is_none(),
            "cannot transfer out of bridge account; BridgeUnlock must be used",
        );

        transfer_check_stateful(self, state, from)
            .await
            .context("stateful transfer check failed")
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        let fee = state
            .get_transfer_base_fee()
            .await
            .context("failed to get transfer base fee")?;
        state
            .get_and_increase_block_fees(&self.fee_asset, fee)
            .await
            .context("failed to add to block fees")?;

        // if fee payment asset is same asset as transfer asset, deduct fee
        // from same balance as asset transferred
        if self.asset.to_ibc_prefixed() == self.fee_asset.to_ibc_prefixed() {
            // check_stateful should have already checked this arithmetic
            let payment_amount = self
                .amount
                .checked_add(fee)
                .expect("transfer amount plus fee should not overflow");

            state
                .decrease_balance(from, &self.asset, payment_amount)
                .await
                .context("failed decreasing `from` account balance")?;
            state
                .increase_balance(self.to, &self.asset, self.amount)
                .await
                .context("failed increasing `to` account balance")?;
        } else {
            // otherwise, just transfer the transfer asset and deduct fee from fee asset balance
            // later
            state
                .decrease_balance(from, &self.asset, self.amount)
                .await
                .context("failed decreasing `from` account balance")?;
            state
                .increase_balance(self.to, &self.asset, self.amount)
                .await
                .context("failed increasing `to` account balance")?;

            // deduct fee from fee asset balance
            state
                .decrease_balance(from, &self.fee_asset, fee)
                .await
                .context("failed decreasing `from` account balance for fee payment")?;
        }

        Ok(())
    }
}
