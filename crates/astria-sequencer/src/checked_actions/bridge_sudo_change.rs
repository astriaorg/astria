use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::BridgeSudoChange,
};
use astria_eyre::eyre::{
    bail,
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
pub(crate) struct CheckedBridgeSudoChange {
    action: BridgeSudoChange,
    tx_signer: TransactionSignerAddressBytes,
}

impl CheckedBridgeSudoChange {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn new<S: StateRead>(
        action: BridgeSudoChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self> {
        state
            .ensure_base_prefix(&action.bridge_address)
            .await
            .wrap_err("bridge address has an unsupported prefix")?;
        if let Some(new_sudo_address) = &action.new_sudo_address {
            state
                .ensure_base_prefix(new_sudo_address)
                .await
                .wrap_err("new sudo address has an unsupported prefix")?;
        }
        if let Some(new_withdrawer_address) = &action.new_withdrawer_address {
            state
                .ensure_base_prefix(new_withdrawer_address)
                .await
                .wrap_err("new withdrawer address has an unsupported prefix")?;
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
        // check that the signer of this tx is the authorized sudo address for the bridge account
        let Some(sudo_address) = state
            .get_bridge_account_sudo_address(&self.action.bridge_address)
            .await
            .wrap_err("failed to read bridge account sudo address from storage")?
        else {
            bail!("bridge account does not have an associated sudo address in storage");
        };

        ensure!(
            &sudo_address == self.tx_signer.as_bytes(),
            "transaction signer not authorized to change bridge sudo address",
        );

        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        self.run_mutable_checks(&state).await?;

        if let Some(sudo_address) = self.action.new_sudo_address {
            state
                .put_bridge_account_sudo_address(&self.action.bridge_address, sudo_address)
                .wrap_err("failed to write bridge account sudo address to storage")?;
        }

        if let Some(withdrawer_address) = self.action.new_withdrawer_address {
            state
                .put_bridge_account_withdrawer_address(
                    &self.action.bridge_address,
                    withdrawer_address,
                )
                .wrap_err("failed to write bridge account withdrawer address to storage")?;
        }

        Ok(())
    }

    pub(super) fn action(&self) -> &BridgeSudoChange {
        &self.action
    }
}

impl AssetTransfer for CheckedBridgeSudoChange {
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
    use crate::test_utils::{
        assert_error_contains,
        dummy_bridge_sudo_change,
        Fixture,
        ASTRIA_PREFIX,
        SUDO_ADDRESS_BYTES,
    };

    #[tokio::test]
    async fn should_fail_construction_if_bridge_address_not_base_prefixed() {
        let fixture = Fixture::default_initialized().await;

        let prefix = "different_prefix";
        let action = BridgeSudoChange {
            bridge_address: address_with_prefix([50; ADDRESS_LEN], prefix),
            ..dummy_bridge_sudo_change()
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
    async fn should_fail_construction_if_new_sudo_address_not_base_prefixed() {
        let fixture = Fixture::default_initialized().await;

        let prefix = "different_prefix";
        let action = BridgeSudoChange {
            new_sudo_address: Some(address_with_prefix([50; ADDRESS_LEN], prefix)),
            ..dummy_bridge_sudo_change()
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
        let action = BridgeSudoChange {
            new_withdrawer_address: Some(address_with_prefix([50; ADDRESS_LEN], prefix)),
            ..dummy_bridge_sudo_change()
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
    async fn should_fail_construction_if_bridge_sudo_address_not_set() {
        let fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_sudo_change();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "bridge account does not have an associated sudo address",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_signer_is_not_bridge_sudo_address() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_sudo_change();
        // Store `SUDO_ADDRESS_BYTES` as the sudo address.
        fixture
            .bridge_initializer(action.bridge_address)
            .init()
            .await;
        let tx_signer = [2; ADDRESS_LEN];
        assert_ne!(tx_signer, *SUDO_ADDRESS_BYTES);

        let err = fixture
            .new_checked_action(action, tx_signer)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "transaction signer not authorized to change bridge sudo address",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_is_not_bridge_sudo_address() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_sudo_change();
        let bridge_address = action.bridge_address;
        // Store `SUDO_ADDRESS_BYTES` as the sudo address.
        fixture.bridge_initializer(bridge_address).init().await;

        // Construct two checked bridge sudo change actions while the sudo address is still the
        // `SUDO_ADDRESS_BYTES` so construction succeeds.
        let checked_action_1: CheckedBridgeSudoChange = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        let checked_action_2: CheckedBridgeSudoChange = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Execute the first checked action to change the sudo address to one different from the tx
        // signer address.
        checked_action_1.execute(fixture.state_mut()).await.unwrap();
        let new_sudo_address = fixture
            .state()
            .get_bridge_account_sudo_address(&bridge_address)
            .await
            .expect("should get bridge sudo address")
            .expect("bridge sudo address should be Some");
        assert_ne!(*SUDO_ADDRESS_BYTES, new_sudo_address);

        // Try to execute the second checked action now - should fail due to signer no longer being
        // authorized.
        let err = checked_action_2
            .execute(fixture.state_mut())
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "transaction signer not authorized to change bridge sudo address",
        );
    }

    #[tokio::test]
    async fn should_execute() {
        let mut fixture = Fixture::default_initialized().await;

        let action = dummy_bridge_sudo_change();
        let bridge_address = action.bridge_address;
        let new_sudo_address = action.new_sudo_address.unwrap();
        let new_withdrawer_address = action.new_withdrawer_address.unwrap();
        fixture.bridge_initializer(bridge_address).init().await;
        let checked_action: CheckedBridgeSudoChange = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        checked_action.execute(fixture.state_mut()).await.unwrap();

        assert_eq!(
            fixture
                .state()
                .get_bridge_account_sudo_address(&bridge_address)
                .await
                .unwrap(),
            Some(new_sudo_address.bytes()),
        );
        assert_eq!(
            fixture
                .state()
                .get_bridge_account_withdrawer_address(&bridge_address)
                .await
                .unwrap(),
            Some(new_withdrawer_address.bytes()),
        );
    }
}
