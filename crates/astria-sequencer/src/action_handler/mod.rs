//! Contains the `ActionHandler` trait, which houses all stateless/stateful checks and execution, as
//! well as all of its implementations.

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

use crate::{
    accounts::{
        AddressBytes,
        StateReadExt as _,
        StateWriteExt as _,
    },
    address::StateReadExt as _,
};

pub(crate) mod impls;

/// This trait is a verbatim copy of `cnidarium_component::ActionHandler`.
///
/// It's duplicated here because all actions are foreign types, forbidding
/// the implementation of [`cnidarium_component::ActionHandler`][1] for
/// these types due to Rust orphan rules.
///
/// [1]: https://github.com/penumbra-zone/penumbra/blob/14959350abcb8cfbf33f9aedc7463fccfd8e3f9f/crates/cnidarium-component/src/action_handler.rs#L30
#[async_trait::async_trait]
pub(crate) trait ActionHandler {
    // Commenting out for the time being as this is currently not being used. Leaving this in
    // for reference as this is copied from cnidarium_component.
    // ```
    // type CheckStatelessContext: Clone + Send + Sync + 'static;
    // async fn check_stateless(&self, context: Self::CheckStatelessContext) -> anyhow::Result<()>;
    // async fn check_historical<S: StateRead + 'static>(&self, _state: Arc<S>) -> anyhow::Result<()> {
    //     Ok(())
    // }
    // ```

    async fn check_stateless(&self) -> astria_eyre::eyre::Result<()>;

    async fn check_and_execute<S: StateWrite>(&self, mut state: S)
        -> astria_eyre::eyre::Result<()>;
}

async fn execute_transfer<S, TAddress>(
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

async fn check_transfer<S, TAddress>(action: &Transfer, from: &TAddress, state: &S) -> Result<()>
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
