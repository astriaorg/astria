use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::InitBridgeAccount,
    Protobuf as _,
};
use astria_eyre::eyre::{
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use cnidarium::StateWrite;

use crate::{
    accounts::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    address::StateReadExt as _,
    app::ActionHandler,
    assets::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    bridge::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for InitBridgeAccount {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        if let Some(withdrawer_address) = &self.withdrawer_address {
            state
                .ensure_base_prefix(withdrawer_address)
                .await
                .wrap_err("failed check for base prefix of withdrawer address")?;
        }
        if let Some(sudo_address) = &self.sudo_address {
            state
                .ensure_base_prefix(sudo_address)
                .await
                .wrap_err("failed check for base prefix of sudo address")?;
        }

        ensure!(
            state.is_allowed_fee_asset(&self.fee_asset).await?,
            "invalid fee asset",
        );

        let fee = state
            .get_init_bridge_account_base_fee()
            .await
            .wrap_err("failed to get base fee for initializing bridge account")?;

        // this prevents the address from being registered as a bridge account
        // if it's been previously initialized as a bridge account.
        //
        // however, there is no prevention of initializing an account as a bridge
        // account that's already been used as a normal EOA.
        //
        // the implication is that the account might already have a balance, nonce, etc.
        // before being converted into a bridge account.
        //
        // after the account becomes a bridge account, it can no longer receive funds
        // via `TransferAction`, only via `BridgeLockAction`.
        if state
            .get_bridge_account_rollup_id(&from)
            .await
            .wrap_err("failed getting rollup ID of bridge account")?
            .is_some()
        {
            bail!("bridge account already exists");
        }

        let balance = state
            .get_account_balance(&from, &self.fee_asset)
            .await
            .wrap_err("failed getting `from` account balance for fee payment")?;

        ensure!(
            balance >= fee,
            "insufficient funds for bridge account initialization",
        );

        state
            .put_bridge_account_rollup_id(&from, self.rollup_id)
            .wrap_err("failed to put bridge account rollup id")?;
        state
            .put_bridge_account_ibc_asset(&from, &self.asset)
            .wrap_err("failed to put asset ID")?;
        state
            .put_bridge_account_sudo_address(&from, self.sudo_address.map_or(from, Address::bytes))
            .wrap_err("failed to put bridge account sudo address")?;
        state
            .put_bridge_account_withdrawer_address(
                &from,
                self.withdrawer_address.map_or(from, Address::bytes),
            )
            .wrap_err("failed to put bridge account withdrawer address")?;
        state
            .get_and_increase_block_fees(&self.fee_asset, fee, Self::full_name())
            .await
            .wrap_err("failed to get and increase block fees")?;
        state
            .decrease_balance(&from, &self.fee_asset, fee)
            .await
            .wrap_err("failed to deduct fee from account balance")?;
        Ok(())
    }
}
