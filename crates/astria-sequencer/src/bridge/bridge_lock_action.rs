use astria_core::{
    protocol::transaction::v1alpha1::action::{
        BridgeLockAction,
        TransferAction,
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use astria_eyre::eyre::{
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use cnidarium::StateWrite;

use crate::{
    accounts::action::{
        check_transfer,
        execute_transfer,
    },
    address::StateReadExt as _,
    app::ActionHandler,
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
    utils::create_deposit_event,
};

#[async_trait::async_trait]
impl ActionHandler for BridgeLockAction {
    async fn check_stateless(&self) -> Result<()> {
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
        // ensure the recipient is a bridge account.
        let rollup_id = state
            .get_bridge_account_rollup_id(self.to)
            .await
            .wrap_err("failed to get bridge account rollup id")?
            .ok_or_eyre("bridge lock must be sent to a bridge account")?;

        let allowed_asset = state
            .get_bridge_account_ibc_asset(self.to)
            .await
            .wrap_err("failed to get bridge account asset ID")?;
        ensure!(
            allowed_asset == self.asset.to_ibc_prefixed(),
            "asset ID is not authorized for transfer to bridge account",
        );

        let transfer_action = TransferAction {
            to: self.to,
            asset: self.asset.clone(),
            amount: self.amount,
            fee_asset: self.fee_asset.clone(),
        };

        check_transfer(&transfer_action, from, &state).await?;
        execute_transfer(&transfer_action, from, &mut state).await?;

        let transaction_id = state
            .get_transaction_context()
            .expect("current source should be set before executing action")
            .transaction_id;
        let source_action_index = state
            .get_transaction_context()
            .expect("current source should be set before executing action")
            .source_action_index;

        let deposit = Deposit::new(
            self.to,
            rollup_id,
            self.amount,
            self.asset.clone(),
            self.destination_chain_address.clone(),
            transaction_id,
            source_action_index,
        );
        let deposit_abci_event = create_deposit_event(&deposit);
        state.record(deposit_abci_event);
        state
            .put_deposit_event(deposit)
            .await
            .wrap_err("failed to put deposit event into state")?;
        Ok(())
    }
}

/// returns the length of a serialized `Deposit` message.
pub(crate) fn get_deposit_byte_len(deposit: &Deposit) -> u128 {
    use prost::Message as _;
    let raw = deposit.clone().into_raw();
    raw.encoded_len() as u128
}
