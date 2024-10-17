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
        protocol::transaction::v1::action::FeeChange,
    };

    use crate::{
        app::ActionHandler as _,
        authority::StateWriteExt as _,
        fees::StateReadExt as _,
        storage::Storage,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    /// This macro generates a test named e.g. `transfer_fee_change_action_executes` which asserts
    /// that executing a `FeeChange` tx for the given action results in the fees being stored for
    /// the given action.
    macro_rules! test_fee_change_action {
        ( $( $fee_name:tt => $fee_ty:tt ),* $(,)?) => {
            $(
                paste::item! {
                    #[tokio::test]
                    async fn [< $fee_name _fee_change_action_executes >] () {
                        use astria_core::protocol::fees::v1:: [< $fee_ty FeeComponents >] as Fees;

                        let storage = Storage::new_temp().await;
                        let mut state_delta = storage.new_delta_of_latest_snapshot();

                        // Put the context to enable the txs to execute.
                        state_delta.put_transaction_context(TransactionContext {
                            address_bytes: [1; 20],
                            transaction_id: TransactionId::new([0; 32]),
                            source_action_index: 0,
                        });
                        state_delta.put_sudo_address([1; 20]).unwrap();

                        assert!(state_delta
                            .[< get_ $fee_name _fees >] ()
                            .await
                            .expect(stringify!(should not error fetching unstored $fee_name fees))
                            .is_none());

                        // Execute an initial fee change tx to store the first version of the fees.
                        let initial_fees = Fees {
                            base: 1,
                            multiplier: 2,
                        };
                        let fee_change = FeeChange:: $fee_ty (initial_fees);
                        fee_change.check_and_execute(&mut state_delta).await.unwrap();

                        let retrieved_fees = state_delta
                            .[< get_ $fee_name _fees >] ()
                            .await
                            .expect(stringify!(should not error fetching initial $fee_name fees))
                            .expect(stringify!(initial $fee_name fees should be stored));
                        assert_eq!(initial_fees, retrieved_fees);

                        // Execute a second fee change tx to overwrite the fees.
                        let new_fees = Fees {
                            base: 3,
                            multiplier: 4,
                        };
                        let fee_change = FeeChange:: $fee_ty (new_fees);
                        fee_change.check_and_execute(&mut state_delta).await.unwrap();

                        let retrieved_fees = state_delta
                            .[< get_ $fee_name _fees >] ()
                            .await
                            .expect(stringify!(should not error fetching new $fee_name fees))
                            .expect(stringify!(new $fee_name fees should be stored));
                        assert_ne!(initial_fees, retrieved_fees);
                        assert_eq!(new_fees, retrieved_fees);
                    }
               }
            )*
        };
    }

    test_fee_change_action!(
        transfer => Transfer,
        rollup_data_submission => RollupDataSubmission,
        ics20_withdrawal => Ics20Withdrawal,
        init_bridge_account => InitBridgeAccount,
        bridge_lock => BridgeLock,
        bridge_unlock => BridgeUnlock,
        bridge_sudo_change => BridgeSudoChange,
        validator_update => ValidatorUpdate,
        ibc_relayer_change => IbcRelayerChange,
        ibc_relay => IbcRelay,
        fee_asset_change => FeeAssetChange,
        fee_change => FeeChange,
        sudo_address_change => SudoAddressChange,
        ibc_sudo_change => IbcSudoChange,
    );
}
