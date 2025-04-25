use astria_core::protocol::transaction::v1::action::FeeChange;
use astria_eyre::eyre::{
    self,
    ensure,
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
    authority::StateReadExt as _,
    fees::StateWriteExt as _,
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for FeeChange {
    async fn check_stateless(&self) -> eyre::Result<()> {
        Ok(())
    }

    /// check that the signer of the transaction is the current sudo address,
    /// as only that address can change the fee
    #[instrument(skip_all, err(level = Level::DEBUG))]
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
                .put_fees(*fees)
                .wrap_err("failed to put transfer fees"),
            Self::RollupDataSubmission(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put sequence fees"),
            Self::Ics20Withdrawal(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put ics20 withdrawal fees"),
            Self::InitBridgeAccount(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put init bridge account fees"),
            Self::BridgeLock(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put bridge lock fees"),
            Self::BridgeUnlock(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put bridge unlock fees"),
            Self::BridgeSudoChange(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put bridge sudo change fees"),
            Self::IbcRelay(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put ibc relay fees"),
            Self::ValidatorUpdate(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put validator update fees"),
            Self::FeeAssetChange(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put fee asset change fees"),
            Self::FeeChange(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put fee change fees"),
            Self::IbcRelayerChange(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put ibc relayer change fees"),
            Self::SudoAddressChange(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put sudo address change fees"),
            Self::IbcSudoChange(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put ibc sudo change fees"),
            Self::BridgeTransfer(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put bridge transfer fees"),
            Self::RecoverIbcClient(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put recover ibc client fees"),
            Self::CurrencyPairsChange(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put currency pairs change fees"),
            Self::MarketsChange(fees) => state
                .put_fees(*fees)
                .wrap_err("failed to put markets change fees"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use astria_core::{
        primitive::v1::TransactionId,
        protocol::{
            fees::v1::*,
            transaction::v1::action::*,
        },
    };
    use astria_eyre::eyre::Report;
    use penumbra_ibc::IbcRelay;

    use crate::{
        accounts::AddressBytes as _,
        action_handler::ActionHandler as _,
        authority::StateWriteExt as _,
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
        },
        fees::{
            FeeHandler,
            StateReadExt as _,
        },
        storage::StoredValue,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn fee_change_action_fails_if_signer_is_not_sudo_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let sudo_address = astria_address(&[0; 20]);
        let signer = astria_address(&[1; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: *signer.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_sudo_address(sudo_address).unwrap();

        let action = FeeChange::Transfer(FeeComponents::<Transfer>::new(1, 2));
        assert_eyre_error(
            &action.check_and_execute(state).await.unwrap_err(),
            "signer is not the sudo key",
        );
    }

    #[tokio::test]
    async fn transfer_fee_change_action_executes_as_expected() {
        test_fee_change_action::<Transfer>().await;
    }

    #[tokio::test]
    async fn rollup_data_submission_fee_change_action_executes_as_expected() {
        test_fee_change_action::<RollupDataSubmission>().await;
    }

    #[tokio::test]
    async fn ics20_withdrawal_fee_change_action_executes_as_expected() {
        test_fee_change_action::<Ics20Withdrawal>().await;
    }

    #[tokio::test]
    async fn init_bridge_account_fee_change_action_executes_as_expected() {
        test_fee_change_action::<InitBridgeAccount>().await;
    }

    #[tokio::test]
    async fn bridge_lock_fee_change_action_executes_as_expected() {
        test_fee_change_action::<BridgeLock>().await;
    }

    #[tokio::test]
    async fn bridge_unlock_fee_change_action_executes_as_expected() {
        test_fee_change_action::<BridgeUnlock>().await;
    }

    #[tokio::test]
    async fn bridge_sudo_change_fee_change_action_executes_as_expected() {
        test_fee_change_action::<BridgeSudoChange>().await;
    }

    #[tokio::test]
    async fn validator_update_fee_change_action_executes_as_expected() {
        test_fee_change_action::<ValidatorUpdate>().await;
    }

    #[tokio::test]
    async fn ibc_relay_fee_change_action_executes_as_expected() {
        test_fee_change_action::<IbcRelay>().await;
    }

    #[tokio::test]
    async fn ibc_relayer_change_fee_change_action_executes_as_expected() {
        test_fee_change_action::<IbcRelayerChange>().await;
    }

    #[tokio::test]
    async fn fee_asset_change_fee_change_action_executes_as_expected() {
        test_fee_change_action::<FeeAssetChange>().await;
    }

    #[tokio::test]
    async fn fee_change_fee_change_action_executes_as_expected() {
        test_fee_change_action::<FeeChange>().await;
    }

    #[tokio::test]
    async fn sudo_address_change_fee_change_action_executes_as_expected() {
        test_fee_change_action::<SudoAddressChange>().await;
    }

    #[tokio::test]
    async fn ibc_sudo_change_fee_change_action_executes_as_expected() {
        test_fee_change_action::<IbcSudoChange>().await;
    }

    #[tokio::test]
    async fn bridge_transfer_fee_change_action_executes_as_expected() {
        test_fee_change_action::<BridgeTransfer>().await;
    }

    #[tokio::test]
    async fn recover_ibc_client_fee_change_action_executes_as_expected() {
        test_fee_change_action::<RecoverIbcClient>().await;
    }

    #[tokio::test]
    async fn currency_pairs_change_fee_change_action_executes_as_expected() {
        test_fee_change_action::<CurrencyPairsChange>().await;
    }

    #[tokio::test]
    async fn markets_change_fee_change_action_executes_as_expected() {
        test_fee_change_action::<MarketsChange>().await;
    }

    async fn test_fee_change_action<'a, F>()
    where
        F: FeeHandler,
        FeeComponents<F>: TryFrom<StoredValue<'a>, Error = Report> + Debug,
        FeeChange: From<FeeComponents<F>>,
    {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        // Put the context to enable the txs to execute.
        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_sudo_address([1; 20]).unwrap();

        assert!(state
            .get_fees::<F>()
            .await
            .expect("should not error fetching unstored action fees")
            .is_none());

        // Execute an initial fee change tx to store the first version of the fees.
        let initial_fees = FeeComponents::<F>::new(1, 2);
        let initial_fee_change = FeeChange::from(initial_fees);
        initial_fee_change
            .check_and_execute(&mut state)
            .await
            .unwrap();

        let retrieved_fees = state
            .get_fees::<F>()
            .await
            .expect("should not error fetching initial action fees")
            .expect("initial action fees should be stored");
        assert_eq!(initial_fees, retrieved_fees);

        // Execute a second fee change tx to overwrite the fees.
        let new_fees = FeeComponents::<F>::new(3, 4);
        let new_fee_change = FeeChange::from(new_fees);
        new_fee_change.check_and_execute(&mut state).await.unwrap();

        let retrieved_fees = state
            .get_fees::<F>()
            .await
            .expect("should not error fetching new action fees")
            .expect("new action fees should be stored");
        assert_ne!(initial_fees, retrieved_fees);
        assert_eq!(new_fees, retrieved_fees);
    }
}
