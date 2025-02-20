use std::sync::Arc;

use astria_core::protocol::genesis::v1::GenesisAppState;
use astria_eyre::eyre::{
    OptionExt as _,
    Result,
    WrapErr as _,
};
use tracing::instrument;

use crate::{
    accounts,
    assets,
    component::{
        Component,
        PrepareStateInfo,
    },
};

#[derive(Default)]
pub(crate) struct AccountsComponent;

#[async_trait::async_trait]
impl Component for AccountsComponent {
    type AppState = GenesisAppState;

    #[instrument(name = "AccountsComponent::init_chain", skip_all, err)]
    async fn init_chain<S>(mut state: S, app_state: &Self::AppState) -> Result<()>
    where
        S: accounts::StateWriteExt + assets::StateReadExt,
    {
        if !app_state.accounts().is_empty() {
            let native_asset = state
                .get_native_asset()
                .await
                .wrap_err("failed to read native asset from state")?
                .ok_or_eyre(
                    "native asset does not exist in state but is required to set genesis account \
                     balances",
                )?;
            for account in app_state.accounts() {
                state
                    .put_account_balance(&account.address, &native_asset, account.balance)
                    .wrap_err("failed writing account balance to state")?;
            }
        }
        Ok(())
    }

    #[instrument(name = "AccountsComponent::prepare_state_for_tx_execution", skip_all)]
    async fn prepare_state_for_tx_execution<S: accounts::StateWriteExt + 'static>(
        _state: &mut Arc<S>,
        _prepare_state_info: &PrepareStateInfo,
    ) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "AccountsComponent::handle_post_tx_execution", skip_all)]
    async fn handle_post_tx_execution<S: accounts::StateWriteExt + 'static>(
        _state: &mut Arc<S>,
    ) -> Result<()> {
        Ok(())
    }
}
