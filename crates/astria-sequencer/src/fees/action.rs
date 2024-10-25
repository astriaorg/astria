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
    use astria_core::{
        primitive::v1::TransactionId,
        protocol::{
            fees::v1::*,
            transaction::v1::action::*,
        },
    };
    use penumbra_ibc::IbcRelay;

    use crate::{
        app::ActionHandler as _,
        authority::StateWriteExt as _,
        fees::{
            access::FeeComponents,
            FeeHandler,
        },
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    macro_rules! get_default_fees_and_fee_changes {
        ($fee_ty:tt) => {
            paste::item! {
                {
                    let initial_fees = [< $fee_ty FeeComponents >] {
                        base: 1,
                        multiplier: 2,
                    };
                    let initial_fee_change = FeeChange::$fee_ty(initial_fees);
                    let new_fees = [< $fee_ty FeeComponents >] {
                        base: 3,
                        multiplier: 4,
                    };
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
        test_fee_change_action::<Transfer, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "transfer",
        )
        .await;
    }

    #[tokio::test]
    async fn rollup_data_submission_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(RollupDataSubmission);
        test_fee_change_action::<RollupDataSubmission, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "rollup_data_submission",
        )
        .await;
    }

    #[tokio::test]
    async fn ics_20_withdrawal_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(Ics20Withdrawal);
        test_fee_change_action::<Ics20Withdrawal, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "ics_20_withdrawal",
        )
        .await;
    }

    #[tokio::test]
    async fn init_bridge_account_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(InitBridgeAccount);
        test_fee_change_action::<InitBridgeAccount, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "init_bridge_account",
        )
        .await;
    }

    #[tokio::test]
    async fn bridge_lock_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(BridgeLock);
        test_fee_change_action::<BridgeLock, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "bridge_lock",
        )
        .await;
    }

    #[tokio::test]
    async fn bridge_unlock_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(BridgeUnlock);
        test_fee_change_action::<BridgeUnlock, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "bridge_unlock",
        )
        .await;
    }

    #[tokio::test]
    async fn bridge_sudo_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(BridgeSudoChange);
        test_fee_change_action::<BridgeSudoChange, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "bridge_sudo_change",
        )
        .await;
    }

    #[tokio::test]
    async fn validator_update_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(ValidatorUpdate);
        test_fee_change_action::<ValidatorUpdate, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "validator_update",
        )
        .await;
    }

    #[tokio::test]
    async fn ibc_relay_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(IbcRelay);
        test_fee_change_action::<IbcRelay, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "ibc_relay",
        )
        .await;
    }

    #[tokio::test]
    async fn ibc_relayer_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(IbcRelayerChange);
        test_fee_change_action::<IbcRelayerChange, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "ibc_relayer_change",
        )
        .await;
    }

    #[tokio::test]
    async fn fee_asset_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(FeeAssetChange);
        test_fee_change_action::<FeeAssetChange, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "fee_asset_change",
        )
        .await;
    }

    #[tokio::test]
    async fn fee_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(FeeChange);
        test_fee_change_action::<FeeChange, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "fee_change",
        )
        .await;
    }

    #[tokio::test]
    async fn sudo_address_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(SudoAddressChange);
        test_fee_change_action::<SudoAddressChange, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "sudo_address_change",
        )
        .await;
    }

    #[tokio::test]
    async fn ibc_sudo_change_fee_change_action_executes_as_expected() {
        let (initial_fees, initial_fee_change, new_fees, new_fee_change) =
            get_default_fees_and_fee_changes!(IbcSudoChange);
        test_fee_change_action::<IbcSudoChange, _>(
            initial_fees,
            initial_fee_change,
            new_fees,
            new_fee_change,
            "ibc_sudo_change",
        )
        .await;
    }

    async fn test_fee_change_action<F: FeeHandler<FeeComponents = T>, T>(
        initial_fees: T,
        initial_fee_change: FeeChange,
        new_fees: T,
        new_fee_change: FeeChange,
        action_name: &str,
    ) where
        T: FeeComponents + std::fmt::Debug + std::cmp::PartialEq<T>,
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
            F::fee_components(&state)
                .await
                .unwrap_or_else(|_| panic!("should not error fetching unstored {action_name} fees"))
                .is_none()
        );

        // Execute an initial fee change tx to store the first version of the fees.
        initial_fee_change
            .check_and_execute(&mut state)
            .await
            .unwrap();

        let retrieved_fees = F::fee_components(&state)
            .await
            .unwrap_or_else(|_| panic!("should not error fetching initial {action_name} fees"))
            .unwrap_or_else(|| panic!("initial {action_name} fees should be stored"));
        assert_eq!(initial_fees, retrieved_fees);

        // Execute a second fee change tx to overwrite the fees.
        new_fee_change.check_and_execute(&mut state).await.unwrap();

        let retrieved_fees = F::fee_components(&state)
            .await
            .unwrap_or_else(|_| panic!("should not error fetching new {action_name} fees"))
            .unwrap_or_else(|| panic!("new {action_name} fees should be stored"));
        assert_ne!(initial_fees, retrieved_fees);
        assert_eq!(new_fees, retrieved_fees);
    }
}
