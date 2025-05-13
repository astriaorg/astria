use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        Address,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::InitBridgeAccount,
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
    address::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
};

#[derive(Debug)]
pub(crate) struct CheckedInitBridgeAccount {
    action: InitBridgeAccount,
    tx_signer: TransactionSignerAddressBytes,
}

impl CheckedInitBridgeAccount {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn new<S: StateRead>(
        action: InitBridgeAccount,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self> {
        // Run immutable checks for base prefix.
        if let Some(sudo_address) = &action.sudo_address {
            state
                .ensure_base_prefix(sudo_address)
                .await
                .wrap_err("sudo address has an unsupported prefix")?;
        }
        if let Some(withdrawer_address) = &action.withdrawer_address {
            state
                .ensure_base_prefix(withdrawer_address)
                .await
                .wrap_err("withdrawer address has an unsupported prefix")?;
        }

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
        //
        // This prevents the address from being registered as a bridge account if it's been
        // previously initialized as a bridge account.
        //
        // However, there is no prevention of initializing an account as a bridge account that's
        // already been used as a normal EOA.
        //
        // The implication is that the account might already have a balance, nonce, etc. before
        // being converted into a bridge account.
        ensure!(
            state
                .get_bridge_account_rollup_id(&self.tx_signer)
                .await
                .wrap_err("failed to read bridge account rollup id from storage")?
                .is_none(),
            "bridge account already exists",
        );
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        self.run_mutable_checks(&state).await?;
        state
            .put_bridge_account_rollup_id(&self.tx_signer, self.action.rollup_id)
            .wrap_err("failed to write bridge account rollup id to storage")?;
        state
            .put_bridge_account_ibc_asset(&self.tx_signer, &self.action.asset)
            .wrap_err("failed to write bridge account asset to storage")?;
        state
            .put_bridge_account_sudo_address(
                &self.tx_signer,
                self.action
                    .sudo_address
                    .map_or(*self.tx_signer.as_bytes(), Address::bytes),
            )
            .wrap_err("failed to write bridge account sudo address to storage")?;
        state
            .put_bridge_account_withdrawer_address(
                &self.tx_signer,
                self.action
                    .withdrawer_address
                    .map_or(*self.tx_signer.as_bytes(), Address::bytes),
            )
            .wrap_err("failed to write bridge account withdrawer address to storage")
    }

    pub(super) fn action(&self) -> &InitBridgeAccount {
        &self.action
    }
}

impl AssetTransfer for CheckedInitBridgeAccount {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::test_utils::address_with_prefix,
        *,
    };
    use crate::{
        accounts::AddressBytes as _,
        test_utils::{
            assert_error_contains,
            dummy_init_bridge_account,
            Fixture,
            ASTRIA_PREFIX,
            SUDO_ADDRESS,
            SUDO_ADDRESS_BYTES,
        },
    };

    #[tokio::test]
    async fn should_fail_construction_if_new_sudo_address_not_base_prefixed() {
        let fixture = Fixture::default_initialized().await;

        let prefix = "different_prefix";
        let action = InitBridgeAccount {
            sudo_address: Some(address_with_prefix([2; ADDRESS_LEN], prefix)),
            ..dummy_init_bridge_account()
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
    async fn should_fail_construction_if_new_withdrawer_address_not_base_prefixed() {
        let fixture = Fixture::default_initialized().await;

        let prefix = "different_prefix";
        let action = InitBridgeAccount {
            withdrawer_address: Some(address_with_prefix([3; ADDRESS_LEN], prefix)),
            ..dummy_init_bridge_account()
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
    async fn should_fail_construction_if_bridge_account_already_initialized() {
        let mut fixture = Fixture::default_initialized().await;
        fixture.bridge_initializer(*SUDO_ADDRESS).init().await;

        let action = dummy_init_bridge_account();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "bridge account already exists");
    }

    #[tokio::test]
    async fn should_fail_execution_if_bridge_account_already_initialized() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct two checked init bridge account actions while the bridge account doesn't
        // exist so construction succeeds.
        let action = dummy_init_bridge_account();
        let checked_action_1: CheckedInitBridgeAccount = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        let checked_action_2: CheckedInitBridgeAccount = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Execute the first checked action to initialize the bridge account.
        checked_action_1.execute(fixture.state_mut()).await.unwrap();

        // Try to execute the second checked action now - should fail due to bridge account now
        // existing.
        let err = checked_action_2
            .execute(fixture.state_mut())
            .await
            .unwrap_err();

        assert_error_contains(&err, "bridge account already exists");
    }

    #[tokio::test]
    async fn should_execute_using_sudo_address_and_withdrawer_address() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_init_bridge_account();
        let checked_action: CheckedInitBridgeAccount = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        checked_action.execute(fixture.state_mut()).await.unwrap();

        assert_eq!(
            fixture
                .state()
                .get_bridge_account_rollup_id(&*SUDO_ADDRESS_BYTES)
                .await
                .unwrap(),
            Some(action.rollup_id)
        );
        assert_eq!(
            fixture
                .state()
                .get_bridge_account_ibc_asset(&*SUDO_ADDRESS_BYTES)
                .await
                .unwrap(),
            action.asset.to_ibc_prefixed()
        );
        assert_eq!(
            fixture
                .state()
                .get_bridge_account_sudo_address(&*SUDO_ADDRESS_BYTES)
                .await
                .unwrap(),
            Some(*action.sudo_address.unwrap().address_bytes())
        );
        assert_eq!(
            fixture
                .state()
                .get_bridge_account_withdrawer_address(&*SUDO_ADDRESS_BYTES)
                .await
                .unwrap(),
            Some(*action.withdrawer_address.unwrap().address_bytes())
        );
    }

    #[tokio::test]
    async fn should_execute_using_no_sudo_address_and_no_withdrawer_address() {
        let mut fixture = Fixture::default_initialized().await;

        let action = InitBridgeAccount {
            sudo_address: None,
            withdrawer_address: None,
            ..dummy_init_bridge_account()
        };
        let checked_action: CheckedInitBridgeAccount = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        checked_action.execute(fixture.state_mut()).await.unwrap();

        assert_eq!(
            fixture
                .state()
                .get_bridge_account_rollup_id(&*SUDO_ADDRESS_BYTES)
                .await
                .unwrap(),
            Some(action.rollup_id)
        );
        assert_eq!(
            fixture
                .state()
                .get_bridge_account_ibc_asset(&*SUDO_ADDRESS_BYTES)
                .await
                .unwrap(),
            action.asset.to_ibc_prefixed()
        );
        assert_eq!(
            fixture
                .state()
                .get_bridge_account_sudo_address(&*SUDO_ADDRESS_BYTES)
                .await
                .unwrap(),
            Some(*SUDO_ADDRESS_BYTES)
        );
        assert_eq!(
            fixture
                .state()
                .get_bridge_account_withdrawer_address(&*SUDO_ADDRESS_BYTES)
                .await
                .unwrap(),
            Some(*SUDO_ADDRESS_BYTES)
        );
    }
}
