use astria_core::protocol::transaction::v1::action::BridgeSudoChange;
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
    bridge::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};
#[async_trait::async_trait]
impl ActionHandler for BridgeSudoChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.bridge_address)
            .await
            .wrap_err("failed check for base prefix of bridge address")?;
        if let Some(new_sudo_address) = &self.new_sudo_address {
            state
                .ensure_base_prefix(new_sudo_address)
                .await
                .wrap_err("failed check for base prefix of new sudo address")?;
        }
        if let Some(new_withdrawer_address) = &self.new_withdrawer_address {
            state
                .ensure_base_prefix(new_withdrawer_address)
                .await
                .wrap_err("failed check for base prefix of new withdrawer address")?;
        }

        // check that the sender of this tx is the authorized sudo address for the bridge account
        let Some(sudo_address) = state
            .get_bridge_account_sudo_address(&self.bridge_address)
            .await
            .wrap_err("failed to get bridge account sudo address")?
        else {
            // TODO: if the sudo address is unset, should we still allow this action
            // if the sender if the bridge address itself?
            bail!("bridge account does not have an associated sudo address");
        };

        ensure!(
            sudo_address == from,
            "unauthorized for bridge sudo change action",
        );

        if let Some(sudo_address) = self.new_sudo_address {
            state
                .put_bridge_account_sudo_address(&self.bridge_address, sudo_address)
                .wrap_err("failed to put bridge account sudo address")?;
        }

        if let Some(withdrawer_address) = self.new_withdrawer_address {
            state
                .put_bridge_account_withdrawer_address(&self.bridge_address, withdrawer_address)
                .wrap_err("failed to put bridge account withdrawer address")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::{
            asset,
            TransactionId,
        },
        protocol::fees::v1::BridgeSudoChangeFeeComponents,
    };

    use super::*;
    use crate::{
        accounts::StateWriteExt as _,
        address::StateWriteExt as _,
        benchmark_and_test_utils::{
            astria_address,
            ASTRIA_PREFIX,
        },
        fees::StateWriteExt as _,
        storage::Storage,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    fn test_asset() -> asset::Denom {
        "test".parse().unwrap()
    }

    #[tokio::test]
    async fn fails_with_unauthorized_if_signer_is_not_sudo_address() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta.put_transaction_context(TransactionContext {
            address_bytes: [1; 20],
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });
        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();

        let asset = test_asset();
        state_delta.put_allowed_fee_asset(&asset).unwrap();

        let bridge_address = astria_address(&[99; 20]);
        let sudo_address = astria_address(&[98; 20]);
        state_delta
            .put_bridge_account_sudo_address(&bridge_address, sudo_address)
            .unwrap();

        let action = BridgeSudoChange {
            bridge_address,
            new_sudo_address: None,
            new_withdrawer_address: None,
            fee_asset: asset.clone(),
        };

        assert!(
            action
                .check_and_execute(state_delta)
                .await
                .unwrap_err()
                .to_string()
                .contains("unauthorized for bridge sudo change action")
        );
    }

    #[tokio::test]
    async fn executes() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let sudo_address = astria_address(&[98; 20]);
        state_delta.put_transaction_context(TransactionContext {
            address_bytes: sudo_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });
        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();
        state_delta
            .put_bridge_sudo_change_fees(BridgeSudoChangeFeeComponents {
                base: 10,
                multiplier: 0,
            })
            .unwrap();

        let fee_asset = test_asset();
        state_delta.put_allowed_fee_asset(&fee_asset).unwrap();

        let bridge_address = astria_address(&[99; 20]);

        state_delta
            .put_bridge_account_sudo_address(&bridge_address, sudo_address)
            .unwrap();

        let new_sudo_address = astria_address(&[98; 20]);
        let new_withdrawer_address = astria_address(&[97; 20]);
        state_delta
            .put_account_balance(&bridge_address, &fee_asset, 10)
            .unwrap();

        let action = BridgeSudoChange {
            bridge_address,
            new_sudo_address: Some(new_sudo_address),
            new_withdrawer_address: Some(new_withdrawer_address),
            fee_asset,
        };

        action.check_and_execute(&mut state_delta).await.unwrap();

        assert_eq!(
            state_delta
                .get_bridge_account_sudo_address(&bridge_address)
                .await
                .unwrap(),
            Some(new_sudo_address.bytes()),
        );
        assert_eq!(
            state_delta
                .get_bridge_account_withdrawer_address(&bridge_address)
                .await
                .unwrap(),
            Some(new_withdrawer_address.bytes()),
        );
    }
}
