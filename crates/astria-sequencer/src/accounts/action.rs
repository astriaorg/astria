use astria_core::protocol::transaction::v1::action::Transfer;
use astria_eyre::eyre::{
    ensure,
    Result,
    WrapErr as _,
};
use cnidarium::{
    StateRead,
    StateWrite,
};

use super::AddressBytes;
use crate::{
    accounts::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    address::StateReadExt as _,
    app::ActionHandler,
    bridge::StateReadExt as _,
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for Transfer {
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
                .get_bridge_account_rollup_id(&from)
                .await
                .wrap_err("failed to get bridge account rollup id")?
                .is_none(),
            "cannot transfer out of bridge account; BridgeUnlock must be used",
        );

        check_transfer(self, &from, &state).await?;
        execute_transfer(self, &from, state).await?;

        Ok(())
    }
}

pub(crate) async fn execute_transfer<S, TAddress>(
    action: &Transfer,
    from: &TAddress,
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
        .increase_balance(&action.to, &action.asset, action.amount)
        .await
        .wrap_err("failed increasing `to` account balance")?;

    Ok(())
}

pub(crate) async fn check_transfer<S, TAddress>(
    action: &Transfer,
    from: &TAddress,
    state: &S,
) -> Result<()>
where
    S: StateRead,
    TAddress: AddressBytes,
{
    state.ensure_base_prefix(&action.to).await.wrap_err(
        "failed ensuring that the destination address matches the permitted base prefix",
    )?;

    let transfer_asset = &action.asset;

    let from_transfer_balance = state
        .get_account_balance(from, transfer_asset)
        .await
        .wrap_err("failed to get account balance in transfer check")?;
    ensure!(
        from_transfer_balance >= action.amount,
        "insufficient funds for transfer"
    );

    Ok(())
}
