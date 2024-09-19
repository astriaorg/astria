use astria_core::protocol::transaction::v1alpha1::action::TransferAction;
use astria_eyre::eyre::{
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::{
    instrument,
    Level,
};

use super::AddressBytes;
use crate::{
    accounts::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    address::StateReadExt as _,
    app::{
        ActionHandler,
        FeeHandler,
    },
    assets::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    bridge::StateReadExt as _,
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for TransferAction {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();

        ensure!(
            state
                .get_bridge_account_rollup_id(from)
                .await
                .wrap_err("failed to get bridge account rollup id")?
                .is_none(),
            "cannot transfer out of bridge account; BridgeUnlock must be used",
        );

        check_transfer(self, from, &state).await?;
        execute_transfer(self, from, state).await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl FeeHandler for TransferAction {
    // allow: false positive due to proc macro; fixed with rust/clippy 1.81
    #[allow(clippy::blocks_in_conditions)]
    #[instrument(skip_all, err(level = Level::WARN))]
    async fn calculate_and_pay_fees<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let tx_context = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action");
        let from = tx_context.address_bytes();
        let fee = state
            .get_transfer_base_fee()
            .await
            .wrap_err("failed to get transfer base fee")?;

        ensure!(
            state
                .is_allowed_fee_asset(&self.fee_asset)
                .await
                .wrap_err("failed to check allowed fee assets in state")?,
            "invalid fee asset",
        );

        state
            .decrease_balance(from, &self.fee_asset, fee)
            .await
            .wrap_err("failed to decrease balance for fee payment")?;
        state.add_fee_to_block_fees(
            self.fee_asset.clone(),
            fee,
            tx_context.transaction_id,
            tx_context.source_action_index,
        )?;

        Ok(())
    }
}

pub(crate) async fn execute_transfer<S, TAddress>(
    action: &TransferAction,
    from: TAddress,
    mut state: S,
) -> Result<()>
where
    S: StateWrite,
    TAddress: AddressBytes,
{
    let from = from.address_bytes();
    state
        .decrease_balance(from, &action.asset, action.amount)
        .await
        .wrap_err("failed decreasing `from` account balance")?;
    state
        .increase_balance(action.to, &action.asset, action.amount)
        .await
        .wrap_err("failed increasing `to` account balance")?;

    Ok(())
}

pub(crate) async fn check_transfer<S, TAddress>(
    action: &TransferAction,
    from: TAddress,
    state: &S,
) -> Result<()>
where
    S: StateRead,
    TAddress: AddressBytes,
{
    state.ensure_base_prefix(&action.to).await.wrap_err(
        "failed ensuring that the destination address matches the permitted base prefix",
    )?;
    ensure!(
        state
            .is_allowed_fee_asset(&action.fee_asset)
            .await
            .wrap_err("failed to check allowed fee assets in state")?,
        "invalid fee asset",
    );

    let fee = state
        .get_transfer_base_fee()
        .await
        .wrap_err("failed to get transfer base fee")?;
    let transfer_asset = action.asset.clone();

    let from_fee_balance = state
        .get_account_balance(&from, &action.fee_asset)
        .await
        .wrap_err("failed getting `from` account balance for fee payment")?;

    // if fee asset is same as transfer asset, ensure accounts has enough funds
    // to cover both the fee and the amount transferred
    if action.fee_asset.to_ibc_prefixed() == transfer_asset.to_ibc_prefixed() {
        let payment_amount = action
            .amount
            .checked_add(fee)
            .ok_or_eyre("transfer amount plus fee overflowed")?;

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
            .wrap_err("failed to get account balance in transfer check")?;
        ensure!(
            from_transfer_balance >= action.amount,
            "insufficient funds for transfer"
        );
    }

    Ok(())
}
