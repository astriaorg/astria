use astria_core::protocol::transaction::v1::action::{
    FeeAssetChange,
    FeeChange,
};
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
};
use cnidarium::StateWrite;

use crate::{
    app::ActionHandler,
    authority::StateReadExt as _,
    fees::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for FeeChange {
    async fn check_stateless(&self) -> eyre::Result<()> {
        Ok(())
    }

    /// check that the signer of the transaction is the current sudo address,
    /// as only that address can change the fee
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> eyre::Result<()> {
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

        match self {
            Self::Transfer(fees) => state
                .put_transfer_fees(*fees)
                .wrap_err("failed to put transfer fees"),
            Self::RollupDataSubmission(fees) => state
                .put_rollup_data_submission_fees(*fees)
                .wrap_err("failed to put sequence fees"),
            Self::Ics20Withdrawal(fees) => state
                .put_ics20_withdrawal_fees(*fees)
                .wrap_err("failed to put ics20 withdrawal fees"),
            Self::InitBridgeAccount(fees) => state
                .put_init_bridge_account_fees(*fees)
                .wrap_err("failed to put init bridge account fees"),
            Self::BridgeLock(fees) => state
                .put_bridge_lock_fees(*fees)
                .wrap_err("failed to put bridge lock fees"),
            Self::BridgeUnlock(fees) => state
                .put_bridge_unlock_fees(*fees)
                .wrap_err("failed to put bridge unlock fees"),
            Self::BridgeSudoChange(fees) => state
                .put_bridge_sudo_change_fees(*fees)
                .wrap_err("failed to put bridge sudo change fees"),
            Self::IbcRelay(fees) => state
                .put_ibc_relay_fees(*fees)
                .wrap_err("failed to put ibc relay fees"),
            Self::ValidatorUpdate(fees) => state
                .put_validator_update_fees(*fees)
                .wrap_err("failed to put validator update fees"),
            Self::FeeAssetChange(fees) => state
                .put_fee_asset_change_fees(*fees)
                .wrap_err("failed to put fee asset change fees"),
            Self::FeeChange(fees) => state
                .put_fee_change_fees(*fees)
                .wrap_err("failed to put fee change fees"),
            Self::IbcRelayerChange(fees) => state
                .put_ibc_relayer_change_fees(*fees)
                .wrap_err("failed to put ibc relayer change fees"),
            Self::SudoAddressChange(fees) => state
                .put_sudo_address_change_fees(*fees)
                .wrap_err("failed to put sudo address change fees"),
            Self::IbcSudoChange(fees) => state
                .put_ibc_sudo_change_fees(*fees)
                .wrap_err("failed to put ibc sudo change fees"),
        }
    }
}

#[async_trait::async_trait]
impl ActionHandler for FeeAssetChange {
    async fn check_stateless(&self) -> eyre::Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> eyre::Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        let authority_sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get authority sudo address")?;
        ensure!(
            authority_sudo_address == from,
            "unauthorized address for fee asset change"
        );
        match self {
            FeeAssetChange::Addition(asset) => {
                state
                    .put_allowed_fee_asset(asset)
                    .context("failed to write allowed fee asset to state")?;
            }
            FeeAssetChange::Removal(asset) => {
                state.delete_allowed_fee_asset(asset);

                if state
                    .get_allowed_fee_assets()
                    .await
                    .wrap_err("failed to retrieve allowed fee assets")?
                    .is_empty()
                {
                    bail!("cannot remove last allowed fee asset");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::TransactionId,
        protocol::{
            fees::v1::{
                BridgeLockFeeComponents,
                Ics20WithdrawalFeeComponents,
                InitBridgeAccountFeeComponents,
                RollupDataSubmissionFeeComponents,
                TransferFeeComponents,
            },
            transaction::v1::action::FeeChange,
        },
    };
    use cnidarium::StateDelta;

    use crate::{
        app::ActionHandler as _,
        authority::StateWriteExt as _,
        fees::{
            StateReadExt as _,
            StateWriteExt as _,
        },
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

        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });
        state.put_sudo_address([1; 20]).unwrap();

        state
            .put_transfer_fees(TransferFeeComponents {
                base: transfer_fee,
                multiplier: 0,
            })
            .unwrap();

        let fee_change = FeeChange::Transfer(TransferFeeComponents {
            base: 10,
            multiplier: 0,
        });

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(state.get_transfer_fees().await.unwrap().base, 10);

        let rollup_data_submission_base = 5;
        let rollup_data_submission_cost_multiplier = 2;
        state
            .put_rollup_data_submission_fees(RollupDataSubmissionFeeComponents {
                base: rollup_data_submission_base,
                multiplier: rollup_data_submission_cost_multiplier,
            })
            .unwrap();

        let fee_change = FeeChange::RollupDataSubmission(RollupDataSubmissionFeeComponents {
            base: 3,
            multiplier: 4,
        });

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(
            state.get_rollup_data_submission_fees().await.unwrap().base,
            3
        );
        assert_eq!(
            state
                .get_rollup_data_submission_fees()
                .await
                .unwrap()
                .multiplier,
            4
        );

        let init_bridge_account_base = 1;
        state
            .put_init_bridge_account_fees(InitBridgeAccountFeeComponents {
                base: init_bridge_account_base,
                multiplier: 0,
            })
            .unwrap();

        let fee_change = FeeChange::InitBridgeAccount(InitBridgeAccountFeeComponents {
            base: 2,
            multiplier: 0,
        });

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(state.get_init_bridge_account_fees().await.unwrap().base, 2);

        let bridge_lock_cost_multiplier = 1;
        state
            .put_bridge_lock_fees(BridgeLockFeeComponents {
                base: 0,
                multiplier: bridge_lock_cost_multiplier,
            })
            .unwrap();

        let fee_change = FeeChange::BridgeLock(BridgeLockFeeComponents {
            base: 0,
            multiplier: 2,
        });

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(state.get_bridge_lock_fees().await.unwrap().multiplier, 2);

        let ics20_withdrawal_base = 1;
        state
            .put_ics20_withdrawal_fees(Ics20WithdrawalFeeComponents {
                base: ics20_withdrawal_base,
                multiplier: 0,
            })
            .unwrap();

        let fee_change = FeeChange::Ics20Withdrawal(Ics20WithdrawalFeeComponents {
            base: 2,
            multiplier: 0,
        });

        fee_change.check_and_execute(&mut state).await.unwrap();
        assert_eq!(state.get_ics20_withdrawal_fees().await.unwrap().base, 2);
    }
}
