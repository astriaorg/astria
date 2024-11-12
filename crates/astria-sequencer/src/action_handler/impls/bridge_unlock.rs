use astria_core::protocol::transaction::v1::action::{
    BridgeUnlock,
    Transfer,
};
use astria_eyre::eyre::{
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;

use crate::{
    action_handler::{
        check_transfer,
        execute_transfer,
        ActionHandler,
    },
    address::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt,
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for BridgeUnlock {
    // TODO(https://github.com/astriaorg/astria/issues/1430): move checks to the `BridgeUnlock` parsing.
    async fn check_stateless(&self) -> Result<()> {
        ensure!(self.amount > 0, "amount must be greater than zero",);
        ensure!(self.memo.len() <= 64, "memo must not be more than 64 bytes");
        ensure!(
            !self.rollup_withdrawal_event_id.is_empty(),
            "rollup withdrawal event id must be non-empty",
        );
        ensure!(
            self.rollup_withdrawal_event_id.len() <= 256,
            "rollup withdrawal event id must not be more than 256 bytes",
        );
        ensure!(
            self.rollup_block_number > 0,
            "rollup block number must be greater than zero",
        );
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.to)
            .await
            .wrap_err("failed check for base prefix of destination address")?;
        state
            .ensure_base_prefix(&self.bridge_address)
            .await
            .wrap_err("failed check for base prefix of bridge address")?;

        let asset = state
            .get_bridge_account_ibc_asset(&self.bridge_address)
            .await
            .wrap_err("failed to get bridge's asset id, must be a bridge account")?;

        // check that the sender of this tx is the authorized withdrawer for the bridge account
        let Some(withdrawer_address) = state
            .get_bridge_account_withdrawer_address(&self.bridge_address)
            .await
            .wrap_err("failed to get bridge account withdrawer address")?
        else {
            bail!("bridge account does not have an associated withdrawer address");
        };

        ensure!(
            withdrawer_address == from,
            "unauthorized to unlock bridge account",
        );

        let transfer_action = Transfer {
            to: self.to,
            asset: asset.into(),
            amount: self.amount,
            fee_asset: self.fee_asset.clone(),
        };

        check_transfer(&transfer_action, &self.bridge_address, &state).await?;
        state
            .check_and_set_withdrawal_event_block_for_bridge_account(
                &self.bridge_address,
                &self.rollup_withdrawal_event_id,
                self.rollup_block_number,
            )
            .await
            .context("withdrawal event already processed")?;
        execute_transfer(&transfer_action, &self.bridge_address, state).await?;

        Ok(())
    }
}
