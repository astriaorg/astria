use astria_core::protocol::transaction::v1::action::IbcSudoChange;
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
    action_handler::ActionHandler,
    address::StateReadExt as _,
    authority::StateReadExt as _,
    ibc::StateWriteExt as _,
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for IbcSudoChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.new_address)
            .await
            .wrap_err("desired new ibc sudo address has an unsupported prefix")?;
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");
        state
            .put_ibc_sudo_address(self.new_address)
            .wrap_err("failed to put ibc sudo address in state")?;
        Ok(())
    }
}
