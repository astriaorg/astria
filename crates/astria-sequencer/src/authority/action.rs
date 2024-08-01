use anyhow::{
    bail,
    ensure,
    Context as _,
    Result,
};
use astria_core::protocol::transaction::v1alpha1::action::{
    FeeChange,
    FeeChangeAction,
    SudoAddressChangeAction,
    ValidatorUpdate,
};
use cnidarium::StateWrite;

use crate::{
    accounts::StateWriteExt as _,
    address::StateReadExt as _,
    app::ActionHandler,
    authority::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    bridge::StateWriteExt as _,
    ibc::StateWriteExt as _,
    sequence::StateWriteExt as _,
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for ValidatorUpdate {
    type CheckStatelessContext = ();

    async fn check_stateless(&self, _context: Self::CheckStatelessContext) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_current_source()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .context("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");

        // ensure that we're not removing the last validator or a validator
        // that doesn't exist, these both cause issues in cometBFT
        if self.power == 0 {
            let validator_set = state
                .get_validator_set()
                .await
                .context("failed to get validator set from state")?;
            // check that validator exists
            if validator_set
                .get(self.verification_key.address_bytes())
                .is_none()
            {
                bail!("cannot remove a non-existing validator");
            }
            // check that this is not the only validator, cannot remove the last one
            ensure!(validator_set.len() != 1, "cannot remove the last validator");
        }

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
    type CheckStatelessContext = ();

    async fn check_stateless(&self, _context: Self::CheckStatelessContext) -> Result<()> {
        Ok(())
    }

    /// check that the signer of the transaction is the current sudo address,
    /// as only that address can change the sudo address
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_current_source()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.new_address)
            .await
            .context("desired new sudo address has an unsupported prefix")?;
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .context("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");
        state
            .put_sudo_address(self.new_address)
            .context("failed to put sudo address in state")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ActionHandler for FeeChangeAction {
    type CheckStatelessContext = ();

    async fn check_stateless(&self, _context: Self::CheckStatelessContext) -> Result<()> {
        Ok(())
    }

    /// check that the signer of the transaction is the current sudo address,
    /// as only that address can change the fee
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_current_source()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .context("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");

        match self.fee_change {
            FeeChange::TransferBaseFee => {
                state
                    .put_transfer_base_fee(self.new_value)
                    .context("failed to put transfer base fee in state")?;
            }
            FeeChange::SequenceBaseFee => state.put_sequence_action_base_fee(self.new_value),
            FeeChange::SequenceByteCostMultiplier => {
                state.put_sequence_action_byte_cost_multiplier(self.new_value);
            }
            FeeChange::InitBridgeAccountBaseFee => {
                state.put_init_bridge_account_base_fee(self.new_value);
            }
            FeeChange::BridgeLockByteCostMultiplier => {
                state.put_bridge_lock_byte_cost_multiplier(self.new_value);
            }
            FeeChange::BridgeSudoChangeBaseFee => {
                state.put_bridge_sudo_change_base_fee(self.new_value);
            }
            FeeChange::Ics20WithdrawalBaseFee => {
                state
                    .put_ics20_withdrawal_base_fee(self.new_value)
                    .context("failed to put ics20 withdrawal base fee in state")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        accounts::StateReadExt as _,
        bridge::StateReadExt as _,
        ibc::StateReadExt as _,
        sequence::StateReadExt as _,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn fee_change_action_executes() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);
        let transfer_fee = 12;

        state.put_current_source(TransactionContext {
            address_bytes: [1; 20],
        });
        state.put_sudo_address([1; 20]).unwrap();

        state.put_transfer_base_fee(transfer_fee).unwrap();

        let fee_change = FeeChangeAction {
            fee_change: FeeChange::TransferBaseFee,
            new_value: 10,
        };

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(state.get_transfer_base_fee().await.unwrap(), 10);

        let sequence_base_fee = 5;
        state.put_sequence_action_base_fee(sequence_base_fee);

        let fee_change = FeeChangeAction {
            fee_change: FeeChange::SequenceBaseFee,
            new_value: 3,
        };

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(state.get_sequence_action_base_fee().await.unwrap(), 3);

        let sequence_byte_cost_multiplier = 2;
        state.put_sequence_action_byte_cost_multiplier(sequence_byte_cost_multiplier);

        let fee_change = FeeChangeAction {
            fee_change: FeeChange::SequenceByteCostMultiplier,
            new_value: 4,
        };

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(
            state
                .get_sequence_action_byte_cost_multiplier()
                .await
                .unwrap(),
            4
        );

        let init_bridge_account_base_fee = 1;
        state.put_init_bridge_account_base_fee(init_bridge_account_base_fee);

        let fee_change = FeeChangeAction {
            fee_change: FeeChange::InitBridgeAccountBaseFee,
            new_value: 2,
        };

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(state.get_init_bridge_account_base_fee().await.unwrap(), 2);

        let bridge_lock_byte_cost_multiplier = 1;
        state.put_bridge_lock_byte_cost_multiplier(bridge_lock_byte_cost_multiplier);

        let fee_change = FeeChangeAction {
            fee_change: FeeChange::BridgeLockByteCostMultiplier,
            new_value: 2,
        };

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(
            state.get_bridge_lock_byte_cost_multiplier().await.unwrap(),
            2
        );

        let ics20_withdrawal_base_fee = 1;
        state
            .put_ics20_withdrawal_base_fee(ics20_withdrawal_base_fee)
            .unwrap();

        let fee_change = FeeChangeAction {
            fee_change: FeeChange::Ics20WithdrawalBaseFee,
            new_value: 2,
        };

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(state.get_ics20_withdrawal_base_fee().await.unwrap(), 2);
    }
}
