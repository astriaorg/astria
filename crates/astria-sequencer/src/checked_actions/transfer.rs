use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::Transfer,
};
use astria_eyre::eyre::{
    ensure,
    Result,
    WrapErr as _,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::{
    instrument,
    Level,
};

use super::{
    AssetTransfer,
    TransactionSignerAddressBytes,
};
use crate::{
    accounts::StateWriteExt as _,
    address::StateReadExt as _,
    bridge::StateReadExt as _,
};

#[derive(Debug)]
pub(crate) struct CheckedTransfer {
    action: Transfer,
    tx_signer: TransactionSignerAddressBytes,
}

impl CheckedTransfer {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn new<S: StateRead>(
        action: Transfer,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self> {
        // Run immutable checks for base prefix.
        state
            .ensure_base_prefix(&action.to)
            .await
            .wrap_err("destination address has an unsupported prefix")?;

        let checked_action = Self {
            action,
            tx_signer: tx_signer.into(),
        };
        checked_action.run_mutable_checks(state).await?;

        Ok(checked_action)
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn run_mutable_checks<S: StateRead>(&self, state: S) -> Result<()> {
        // Ensure the tx signer account is not a bridge account.
        ensure!(
            state
                .get_bridge_account_rollup_id(&self.tx_signer)
                .await
                .wrap_err("failed to read bridge account rollup id from storage")?
                .is_none(),
            "cannot transfer out of bridge account; BridgeUnlock or BridgeTransfer must be used",
        );
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        self.run_mutable_checks(&state).await?;

        state
            .decrease_balance(&self.tx_signer, &self.action.asset, self.action.amount)
            .await
            .wrap_err("failed to decrease signer account balance")?;
        state
            .increase_balance(&self.action.to, &self.action.asset, self.action.amount)
            .await
            .wrap_err("failed to increase destination account balance")?;

        Ok(())
    }

    pub(super) fn action(&self) -> &Transfer {
        &self.action
    }
}

impl AssetTransfer for CheckedTransfer {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        Some((self.action.asset.to_ibc_prefixed(), self.action.amount))
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::RollupId,
        protocol::transaction::v1::action::*,
    };

    use super::{
        super::test_utils::address_with_prefix,
        *,
    };
    use crate::{
        checked_actions::CheckedInitBridgeAccount,
        test_utils::{
            assert_error_contains,
            dummy_transfer,
            Fixture,
            ASTRIA_PREFIX,
            SUDO_ADDRESS,
            SUDO_ADDRESS_BYTES,
        },
    };

    #[tokio::test]
    async fn should_fail_construction_if_destination_address_not_base_prefixed() {
        let fixture = Fixture::default_initialized().await;

        let prefix = "different_prefix";
        let action = Transfer {
            to: address_with_prefix([50; ADDRESS_LEN], prefix),
            ..dummy_transfer()
        };
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!("address has prefix `{prefix}` but only `{ASTRIA_PREFIX}` is permitted"),
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_signer_account_is_bridge_account() {
        let mut fixture = Fixture::default_initialized().await;
        fixture.bridge_initializer(*SUDO_ADDRESS).init().await;

        let action = dummy_transfer();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "cannot transfer out of bridge account; BridgeUnlock or BridgeTransfer must be used",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_account_is_bridge_account() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct a checked transfer while the signer account is not a bridge account.
        let action = dummy_transfer();
        let checked_action: CheckedTransfer = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Initialize the signer's account as a bridge account.
        let init_bridge_account = InitBridgeAccount {
            rollup_id: RollupId::new([1; 32]),
            asset: "test".parse().unwrap(),
            fee_asset: "test".parse().unwrap(),
            sudo_address: None,
            withdrawer_address: None,
        };
        let checked_init_bridge_account: CheckedInitBridgeAccount = fixture
            .new_checked_action(init_bridge_account, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_init_bridge_account
            .execute(fixture.state_mut())
            .await
            .unwrap();

        // Try to execute the checked transfer now - should fail due to bridge account now existing.
        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "cannot transfer out of bridge account; BridgeUnlock or BridgeTransfer must be used",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_account_has_insufficient_balance() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_transfer();
        let checked_action: CheckedTransfer = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(&err, "failed to decrease signer account balance");
    }

    #[tokio::test]
    async fn should_execute() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct the checked transfer while the account has insufficient balance to ensure
        // balance checks are only part of execution.
        let action = dummy_transfer();
        let checked_action: CheckedTransfer = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Provide the signer account with sufficient balance.
        fixture
            .state_mut()
            .increase_balance(&*SUDO_ADDRESS_BYTES, &action.asset, action.amount)
            .await
            .unwrap();

        // Check the balances are correct before execution.
        assert_eq!(
            fixture.get_nria_balance(&*SUDO_ADDRESS_BYTES).await,
            action.amount
        );
        assert_eq!(fixture.get_nria_balance(&action.to).await, 0);

        // Execute the transfer.
        checked_action.execute(fixture.state_mut()).await.unwrap();

        // Check the balances are correct after execution.
        assert_eq!(fixture.get_nria_balance(&*SUDO_ADDRESS_BYTES).await, 0);
        assert_eq!(fixture.get_nria_balance(&action.to).await, action.amount);
    }
}
