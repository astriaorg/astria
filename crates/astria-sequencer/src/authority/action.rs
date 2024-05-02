use anyhow::{
    bail,
    ensure,
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::{
        FeeChange,
        FeeChangeAction,
        SudoAddressChangeAction,
    },
};
use tendermint::account;
use tracing::instrument;

use crate::{
    authority::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    transaction::action_handler::ActionHandler,
};

#[async_trait::async_trait]
impl ActionHandler for tendermint::validator::Update {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .context("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");

        // ensure that we're not removing the last validator or a validator
        // that doesn't exist, these both cause issues in cometBFT
        if self.power.is_zero() {
            let validator_set = state
                .get_validator_set()
                .await
                .context("failed to get validator set from state")?;
            // check that validator exists
            if validator_set
                .get(&account::Id::from(self.pub_key))
                .is_none()
            {
                bail!("cannot remove a non-existing validator");
            }
            // check that this is not the only validator, cannot remove the last one
            ensure!(validator_set.len() != 1, "cannot remove the last validator");
        }
        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, _: Address) -> Result<()> {
        // add validator update in non-consensus state to be used in end_block
        let mut validator_updates = state
            .get_validator_updates()
            .await
            .context("failed getting validator updates from state")?;
        validator_updates.push_update(self.clone());
        state
            .put_validator_updates(validator_updates)
            .context("failed to put validator updates in state")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ActionHandler for SudoAddressChangeAction {
    /// check that the signer of the transaction is the current sudo address,
    /// as only that address can change the sudo address
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .context("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");
        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, _: Address) -> Result<()> {
        state
            .put_sudo_address(self.new_address)
            .context("failed to put sudo address in state")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ActionHandler for FeeChangeAction {
    async fn check_stateless(&self) -> Result<()> {
        // ensure fee to change can actually be changed
        match self.fee_change() {
            FeeChange::TransferBaseFee
            | FeeChange::SequenceBaseFee
            | FeeChange::SequenceByteCostMultiplier
            | FeeChange::InitBridgeAccountBaseFee
            | FeeChange::BridgeLockByteCostMultiplier
            | FeeChange::Ics20WithdrawalBaseFee => Ok(()),
            _ => bail!("invalid fee change: {:?}", self.fee_change()),
        }
    }

    /// check that the signer of the transaction is the current sudo address,
    /// as only that address can change the fee
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .context("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");
        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, _: Address) -> Result<()> {
        use crate::{
            accounts::state_ext::StateWriteExt as _,
            bridge::state_ext::StateWriteExt as _,
            ibc::state_ext::StateWriteExt as _,
            sequence::state_ext::StateWriteExt as _,
        };

        match self.fee_change() {
            FeeChange::TransferBaseFee => {
                state
                    .put_transfer_base_fee(self.new_value())
                    .context("failed to put transfer base fee in state")?;
            }
            FeeChange::SequenceBaseFee => state.put_sequence_action_base_fee(self.new_value()),
            FeeChange::SequenceByteCostMultiplier => {
                state.put_sequence_action_byte_cost_multiplier(self.new_value())
            }
            FeeChange::InitBridgeAccountBaseFee => {
                state.put_init_bridge_account_base_fee(self.new_value())
            }
            FeeChange::BridgeLockByteCostMultiplier => {
                state.put_bridge_lock_byte_cost_multiplier(self.new_value())
            }
            FeeChange::Ics20WithdrawalBaseFee => {
                state
                    .put_ics20_withdrawal_base_fee(self.new_value())
                    .context("failed to put ics20 withdrawal base fee in state")?;
            }
            _ => bail!("invalid fee change: {:?}", self.fee_change()),
        }

        Ok(())
    }
}
