use astria_core::protocol::transaction::v1::action::{
    BridgeUnlock,
    Transfer,
};
use astria_eyre::eyre::{
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;
use tracing::{
    instrument,
    Level,
};

use crate::{
    action_handler::{
        check_transfer,
        execute_transfer,
        ActionHandler,
    },
    address::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt,
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for BridgeUnlock {
    // TODO(https://github.com/astriaorg/astria/issues/1430): move checks to the `BridgeUnlock` parsing.
    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_stateless(&self) -> Result<()> {
        ensure!(self.amount > 0, "amount must be greater than zero",);
        ensure!(self.memo.len() <= 64, "memo must not be more than 64 bytes");
        ensure!(
            !self.rollup_withdrawal_event_id.is_empty(),
            "rollup withdrawal event id must be non-empty",
        );
        ensure!(
            self.rollup_withdrawal_event_id.len() <= 256,
            "rollup withdrawal event id must not be more than 256 bytes",
        );
        ensure!(
            self.rollup_block_number > 0,
            "rollup block number must be greater than zero",
        );
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.to)
            .await
            .wrap_err("failed check for base prefix of destination address")?;
        state
            .ensure_base_prefix(&self.bridge_address)
            .await
            .wrap_err("failed check for base prefix of bridge address")?;

        let asset = state
            .get_bridge_account_ibc_asset(&self.bridge_address)
            .await
            .wrap_err("failed to get bridge's asset id, must be a bridge account")?;

        // check that the sender of this tx is the authorized withdrawer for the bridge account
        let Some(withdrawer_address) = state
            .get_bridge_account_withdrawer_address(&self.bridge_address)
            .await
            .wrap_err("failed to get bridge account withdrawer address")?
        else {
            bail!("bridge account does not have an associated withdrawer address");
        };

        ensure!(
            withdrawer_address == from,
            "unauthorized to unlock bridge account",
        );

        let transfer_action = Transfer {
            to: self.to,
            asset: asset.into(),
            amount: self.amount,
            fee_asset: self.fee_asset.clone(),
        };

        check_transfer(&transfer_action, &self.bridge_address, &state).await?;
        state
            .check_and_set_withdrawal_event_block_for_bridge_account(
                &self.bridge_address,
                &self.rollup_withdrawal_event_id,
                self.rollup_block_number,
            )
            .await
            .context("withdrawal event already processed")?;
        execute_transfer(&transfer_action, &self.bridge_address, state).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::{
            Address,
            RollupId,
            TransactionId,
        },
        protocol::transaction::v1::action::BridgeUnlock,
    };
    use cnidarium::StateDelta;

    use crate::{
        accounts::StateWriteExt as _,
        action_handler::{
            impls::test_utils::test_asset,
            ActionHandler as _,
        },
        address::StateWriteExt as _,
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
            nria,
            ASTRIA_PREFIX,
        },
        bridge::StateWriteExt as _,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn bridge_unlock_fails_if_bridge_account_has_no_withdrawer_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let asset = test_asset();
        let transfer_amount = 100;

        let to_address = astria_address(&[2; 20]);
        let bridge_address = astria_address(&[3; 20]);
        state
            .put_bridge_account_ibc_asset(&bridge_address, &asset)
            .unwrap();

        let bridge_unlock = BridgeUnlock {
            to: to_address,
            amount: transfer_amount,
            fee_asset: asset.clone(),
            memo: String::new(),
            bridge_address,
            rollup_block_number: 1,
            rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
        };

        // invalid sender, doesn't match action's `from`, should fail
        assert_eyre_error(
            &bridge_unlock.check_and_execute(state).await.unwrap_err(),
            "bridge account does not have an associated withdrawer address",
        );
    }

    #[tokio::test]
    async fn bridge_unlock_fails_if_withdrawer_is_not_signer() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let asset = test_asset();
        let transfer_amount = 100;

        let to_address = astria_address(&[2; 20]);
        let bridge_address = astria_address(&[3; 20]);
        let withdrawer_address = astria_address(&[4; 20]);
        state
            .put_bridge_account_withdrawer_address(&bridge_address, withdrawer_address)
            .unwrap();
        state
            .put_bridge_account_ibc_asset(&bridge_address, &asset)
            .unwrap();

        let bridge_unlock = BridgeUnlock {
            to: to_address,
            amount: transfer_amount,
            fee_asset: asset,
            memo: String::new(),
            bridge_address,
            rollup_block_number: 1,
            rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
        };

        // invalid sender, doesn't match action's bridge account's withdrawer, should fail
        assert_eyre_error(
            &bridge_unlock.check_and_execute(state).await.unwrap_err(),
            "unauthorized to unlock bridge account",
        );
    }

    #[tokio::test]
    async fn bridge_unlock_executes_with_duplicated_withdrawal_event_id() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let bridge_address = astria_address(&[1; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: bridge_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let asset = test_asset();
        let transfer_amount = 100;

        let to_address = astria_address(&[2; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"test_rollup_id");

        state
            .put_bridge_account_rollup_id(&bridge_address, rollup_id)
            .unwrap();
        state
            .put_bridge_account_ibc_asset(&bridge_address, &asset)
            .unwrap();
        state
            .put_bridge_account_withdrawer_address(&bridge_address, bridge_address)
            .unwrap();
        // Put plenty of balance
        state
            .put_account_balance(&bridge_address, &asset, 3 * transfer_amount)
            .unwrap();

        let bridge_unlock_first = BridgeUnlock {
            to: to_address,
            amount: transfer_amount,
            fee_asset: asset.clone(),
            memo: String::new(),
            bridge_address,
            rollup_block_number: 1,
            rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
        };
        let bridge_unlock_second = BridgeUnlock {
            rollup_block_number: 10,
            ..bridge_unlock_first.clone()
        };

        // first should succeed, next should fail due to duplicate event.
        bridge_unlock_first
            .check_and_execute(&mut state)
            .await
            .unwrap();
        assert_eyre_error(
            &bridge_unlock_second
                .check_and_execute(&mut state)
                .await
                .unwrap_err(),
            "withdrawal event already processed",
        );
    }

    #[tokio::test]
    async fn bridge_unlock_fails_if_destination_address_is_not_base_prefixed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        // Put different base prefix into state
        let different_prefix = "different_prefix";
        state.put_base_prefix(different_prefix.to_string()).unwrap();

        let bridge_lock_action = BridgeUnlock {
            to: astria_address(&[0; 20]), // not base prefixed
            amount: 1,
            fee_asset: nria().into(),
            memo: String::new(),
            bridge_address: astria_address(&[1; 20]),
            rollup_block_number: 1,
            rollup_withdrawal_event_id: "rollup_withdrawal_event_id".to_string(),
        };

        assert_eyre_error(
            &bridge_lock_action
                .check_and_execute(&mut state)
                .await
                .unwrap_err(),
            &format!(
                "address has prefix `{ASTRIA_PREFIX}` but only `{different_prefix}` is permitted"
            ),
        );
    }

    #[tokio::test]
    async fn bridge_unlock_fails_if_bridge_address_is_not_base_prefixed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        // Construct non base-prefixed bridge address
        let different_prefix = "different_prefix";
        let bridge_address = Address::builder()
            .array([1; 20])
            .prefix(different_prefix)
            .try_build()
            .unwrap();

        let bridge_lock_action = BridgeUnlock {
            to: astria_address(&[0; 20]),
            amount: 1,
            fee_asset: nria().into(),
            memo: String::new(),
            bridge_address,
            rollup_block_number: 1,
            rollup_withdrawal_event_id: "rollup_withdrawal_event_id".to_string(),
        };

        assert_eyre_error(
            &bridge_lock_action
                .check_and_execute(&mut state)
                .await
                .unwrap_err(),
            &format!(
                "address has prefix `{different_prefix}` but only `{ASTRIA_PREFIX}` is permitted",
            ),
        );
    }

    #[tokio::test]
    async fn bridge_unlock_fails_if_bridge_address_is_not_a_bridge_account() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let bridge_address = astria_address(&[1; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: bridge_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        // No rollup ID or asset associated with `bridge_address` in state

        let action = BridgeUnlock {
            to: astria_address(&[2; 20]),
            amount: 100,
            fee_asset: nria().into(),
            memo: String::new(),
            bridge_address,
            rollup_block_number: 1,
            rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
        };

        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "failed to get bridge's asset id, must be a bridge account",
        );
    }
}
