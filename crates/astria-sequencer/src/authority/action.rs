use astria_core::protocol::transaction::v1alpha1::action::{
    FeeChange,
    FeeComponents,
    IbcSudoChange,
    SudoAddressChange,
    ValidatorUpdate,
};
use astria_eyre::eyre::{
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use cnidarium::StateWrite;

use crate::{
    address::StateReadExt as _,
    app::ActionHandler,
    authority::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    fees::StateWriteExt as _,
    ibc::StateWriteExt as _,
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for ValidatorUpdate {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");

        // ensure that we're not removing the last validator or a validator
        // that doesn't exist, these both cause issues in cometBFT
        if self.power == 0 {
            let validator_set = state
                .get_validator_set()
                .await
                .wrap_err("failed to get validator set from state")?;
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
            .wrap_err("failed getting validator updates from state")?;
        validator_updates.push_update(self.clone());
        state
            .put_validator_updates(validator_updates)
            .wrap_err("failed to put validator updates in state")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ActionHandler for SudoAddressChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    /// check that the signer of the transaction is the current sudo address,
    /// as only that address can change the sudo address
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.new_address)
            .await
            .wrap_err("desired new sudo address has an unsupported prefix")?;
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");
        state
            .put_sudo_address(self.new_address)
            .wrap_err("failed to put sudo address in state")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ActionHandler for FeeChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    /// check that the signer of the transaction is the current sudo address,
    /// as only that address can change the fee
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");

        match self.fee_change {
            FeeComponents::TransferFeeComponents(fees) => state
                .put_transfer_fees(fees)
                .wrap_err("failed to put transfer fees"),
            FeeComponents::SequenceFeeComponents(fees) => state
                .put_sequence_fees(fees)
                .wrap_err("failed to put sequence fees"),
            FeeComponents::Ics20WithdrawalFeeComponents(fees) => state
                .put_ics20_withdrawal_fees(fees)
                .wrap_err("failed to put ics20 withdrawal fees"),
            FeeComponents::InitBridgeAccountFeeComponents(fees) => state
                .put_init_bridge_account_fees(fees)
                .wrap_err("failed to put init bridge account fees"),
            FeeComponents::BridgeLockFeeComponents(fees) => state
                .put_bridge_lock_fees(fees)
                .wrap_err("failed to put bridge lock fees"),
            FeeComponents::BridgeUnlockFeeComponents(fees) => state
                .put_bridge_unlock_fees(fees)
                .wrap_err("failed to put bridge unlock fees"),
            FeeComponents::BridgeSudoChangeFeeComponents(fees) => state
                .put_bridge_sudo_change_fees(fees)
                .wrap_err("failed to put bridge sudo change base fees"),
            FeeComponents::IbcRelayFeeComponents(fees) => state
                .put_ibc_relay_fees(fees)
                .wrap_err("failed to put ibc relay fees"),
            FeeComponents::ValidatorUpdateFeeComponents(fees) => state
                .put_validator_update_fees(fees)
                .wrap_err("failed to put validator update fees"),
            FeeComponents::FeeAssetChangeFeeComponents(fees) => state
                .put_fee_asset_change_fees(fees)
                .wrap_err("failed to put fee asset change fees"),
            FeeComponents::FeeChangeFeeComponents(fees) => state
                .put_fee_change_fees(fees)
                .wrap_err("failed to put fee change fees"),
            FeeComponents::IbcRelayerChangeFeeComponents(fees) => state
                .put_ibc_relayer_change_fees(fees)
                .wrap_err("failed to put ibc relayer change fees"),
            FeeComponents::SudoAddressChangeFeeComponents(fees) => state
                .put_sudo_address_change_fees(fees)
                .wrap_err("failed to put sudo address change fees"),
            FeeComponents::IbcSudoChangeFeeComponents(fees) => state
                .put_ibc_sudo_change_fees(fees)
                .wrap_err("failed to put ibc sudo change fees"),
        }
    }
}

#[async_trait::async_trait]
impl ActionHandler for IbcSudoChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

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

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::TransactionId,
        protocol::{
            fees::v1alpha1::{
                BridgeLockFeeComponents,
                FeeComponentsInner,
                Ics20WithdrawalFeeComponents,
                InitBridgeAccountFeeComponents,
                SequenceFeeComponents,
                TransferFeeComponents,
            },
            transaction::v1alpha1::action::{
                BridgeLock,
                FeeComponents,
                Ics20Withdrawal,
                InitBridgeAccount,
                Sequence,
                Transfer,
            },
        },
    };
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        fees::FeeHandler as _,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    #[expect(clippy::too_many_lines, reason = "it's a test")]
    async fn fee_change_action_executes() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);
        let transfer_fee = 12;

        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });
        state.put_sudo_address([1; 20]).unwrap();

        state
            .put_transfer_fees(TransferFeeComponents(FeeComponentsInner {
                base_fee: transfer_fee,
                computed_cost_multiplier: 0,
            }))
            .unwrap();

        let fee_change = FeeChange {
            fee_change: FeeComponents::TransferFeeComponents(TransferFeeComponents(
                FeeComponentsInner {
                    base_fee: 10,
                    computed_cost_multiplier: 0,
                },
            )),
        };

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(Transfer::fee_components(&state).await.unwrap().base_fee, 10);

        let sequence_base_fee = 5;
        let sequence_cost_multiplier = 2;
        state
            .put_sequence_fees(SequenceFeeComponents(FeeComponentsInner {
                base_fee: sequence_base_fee,
                computed_cost_multiplier: sequence_cost_multiplier,
            }))
            .unwrap();

        let fee_change = FeeChange {
            fee_change: FeeComponents::SequenceFeeComponents(SequenceFeeComponents(
                FeeComponentsInner {
                    base_fee: 3,
                    computed_cost_multiplier: 4,
                },
            )),
        };

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(Sequence::fee_components(&state).await.unwrap().base_fee, 3);
        assert_eq!(
            Sequence::fee_components(&state)
                .await
                .unwrap()
                .computed_cost_multiplier,
            4
        );

        let init_bridge_account_base_fee = 1;
        state
            .put_init_bridge_account_fees(InitBridgeAccountFeeComponents(FeeComponentsInner {
                base_fee: init_bridge_account_base_fee,
                computed_cost_multiplier: 0,
            }))
            .unwrap();

        let fee_change = FeeChange {
            fee_change: FeeComponents::InitBridgeAccountFeeComponents(
                InitBridgeAccountFeeComponents(FeeComponentsInner {
                    base_fee: 2,
                    computed_cost_multiplier: 0,
                }),
            ),
        };

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(
            InitBridgeAccount::fee_components(&state)
                .await
                .unwrap()
                .base_fee,
            2
        );

        let bridge_lock_cost_multiplier = 1;
        state
            .put_bridge_lock_fees(BridgeLockFeeComponents(FeeComponentsInner {
                base_fee: 0,
                computed_cost_multiplier: bridge_lock_cost_multiplier,
            }))
            .unwrap();

        let fee_change = FeeChange {
            fee_change: FeeComponents::BridgeLockFeeComponents(BridgeLockFeeComponents(
                FeeComponentsInner {
                    base_fee: 0,
                    computed_cost_multiplier: 2,
                },
            )),
        };

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(
            BridgeLock::fee_components(&state)
                .await
                .unwrap()
                .computed_cost_multiplier,
            2
        );

        let ics20_withdrawal_base_fee = 1;
        state
            .put_ics20_withdrawal_fees(Ics20WithdrawalFeeComponents(FeeComponentsInner {
                base_fee: ics20_withdrawal_base_fee,
                computed_cost_multiplier: 0,
            }))
            .unwrap();

        let fee_change = FeeChange {
            fee_change: FeeComponents::Ics20WithdrawalFeeComponents(Ics20WithdrawalFeeComponents(
                FeeComponentsInner {
                    base_fee: 2,
                    computed_cost_multiplier: 0,
                },
            )),
        };

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(
            Ics20Withdrawal::fee_components(state)
                .await
                .unwrap()
                .base_fee,
            2
        );
    }
}
