use astria_core::protocol::transaction::v1::action::Transfer;
use astria_eyre::eyre::{
    ensure,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;
use tracing::{
    instrument,
    Level,
};

use crate::{
    action_handler::{
        check_transfer,
        execute_transfer,
        ActionHandler,
    },
    bridge::StateReadExt as _,
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for Transfer {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
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
