use astria_core::protocol::transaction::v1::action::{
    FeeAssetChange,
    FeeChange,
};
use astria_eyre::eyre::{
    self,
    ensure,
    WrapErr as _,
};
use cnidarium::StateWrite;
use futures::StreamExt;
use tokio::pin;

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

                pin!(
                    let assets = state.allowed_fee_assets();
                );
                ensure!(
                    assets
                        .filter_map(|item| std::future::ready(item.ok()))
                        .next()
                        .await
                        .is_some(),
                    "cannot remove last allowed fee asset",
                );
            }
        }
        Ok(())
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
        app::ActionHandler as _,
        authority::StateWriteExt as _,
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

    macro_rules! get_default_fees_and_fee_changes {
        ($fee_ty:tt) => {
            paste::item! {
                {
                    let initial_fees = [< $fee_ty FeeComponents >] ::new(1, 2);
                    let initial_fee_change = FeeChange::$fee_ty(initial_fees);
                    let new_fees = [< $fee_ty FeeComponents >] ::new(3, 4);
                    let new_fee_change = FeeChange::$fee_ty(new_fees);
                    (initial_fees, initial_fee_change, new_fees, new_fee_change)
                }
            }
        };
    }

    #[tokio::test]
    async fn transfer_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(Transfer);
        test_fee_change_action::<Transfer>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn rollup_data_submission_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(RollupDataSubmission);
        test_fee_change_action::<RollupDataSubmission>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn ics_20_withdrawal_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(Ics20Withdrawal);
        test_fee_change_action::<Ics20Withdrawal>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn init_bridge_account_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(InitBridgeAccount);
        test_fee_change_action::<InitBridgeAccount>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn bridge_lock_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(BridgeLock);
        test_fee_change_action::<BridgeLock>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn bridge_unlock_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(BridgeUnlock);
        test_fee_change_action::<BridgeUnlock>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn bridge_sudo_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(BridgeSudoChange);
        test_fee_change_action::<BridgeSudoChange>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn validator_update_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(ValidatorUpdate);
        test_fee_change_action::<ValidatorUpdate>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn ibc_relay_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(IbcRelay);
        test_fee_change_action::<IbcRelay>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn ibc_relayer_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(IbcRelayerChange);
        test_fee_change_action::<IbcRelayerChange>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn fee_asset_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(FeeAssetChange);
        test_fee_change_action::<FeeAssetChange>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn fee_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(FeeChange);
        test_fee_change_action::<FeeChange>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn sudo_address_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(SudoAddressChange);
        test_fee_change_action::<SudoAddressChange>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    #[tokio::test]
    async fn ibc_sudo_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(IbcSudoChange);
        test_fee_change_action::<IbcSudoChange>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
        )
        .await;
    }

    async fn test_fee_change_action<'a, F>(
        initial_fees: FeeComponents<F>,
        initial_fee_change: FeeChange,
        new_fees: FeeComponents<F>,
        new_fee_change: FeeChange,
    ) where
        F: FeeHandler,
        FeeComponents<F>: TryFrom<StoredValue<'a>, Error = Report> + Debug,
    {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        // Put the context to enable the txs to execute.
        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });
        state.put_sudo_address([1; 20]).unwrap();

        assert!(
            state
                .get_fees::<F>()
                .await
                .expect("should not error fetching unstored action fees")
                .is_none()
        );

        // Execute an initial fee change tx to store the first version of the fees.
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
