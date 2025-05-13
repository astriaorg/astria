use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::IbcRelayerChange,
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
    ibc::{
        StateReadExt as _,
        StateWriteExt as _,
    },
};

#[derive(Debug)]
pub(crate) struct CheckedIbcRelayerChange {
    action: IbcRelayerChange,
    tx_signer: TransactionSignerAddressBytes,
}

impl CheckedIbcRelayerChange {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn new<S: StateRead>(
        action: IbcRelayerChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self> {
        // Run immutable checks for base prefix.
        match &action {
            IbcRelayerChange::Addition(address) | IbcRelayerChange::Removal(address) => {
                state
                    .ensure_base_prefix(address)
                    .await
                    .wrap_err("ibc relayer change address has an unsupported prefix")?;
            }
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
        // Check that the signer of this tx is the authorized IBC sudo address.
        let ibc_sudo_address = state
            .get_ibc_sudo_address()
            .await
            .wrap_err("failed to read ibc sudo address from storage")?;
        ensure!(
            &ibc_sudo_address == self.tx_signer.as_bytes(),
            "transaction signer not authorized to change ibc relayer",
        );

        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        self.run_mutable_checks(&state).await?;

        match self.action {
            IbcRelayerChange::Addition(address) => {
                state
                    .put_ibc_relayer_address(&address)
                    .wrap_err("failed to write ibc relayer address to storage")?;
            }
            IbcRelayerChange::Removal(address) => {
                state.delete_ibc_relayer_address(&address);
            }
        }

        Ok(())
    }

    pub(super) fn action(&self) -> &IbcRelayerChange {
        &self.action
    }
}

impl AssetTransfer for CheckedIbcRelayerChange {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use astria_core::protocol::transaction::v1::action::IbcSudoChange;

    use super::{
        super::test_utils::address_with_prefix,
        *,
    };
    use crate::{
        checked_actions::CheckedIbcSudoChange,
        test_utils::{
            assert_error_contains,
            astria_address,
            Fixture,
            ASTRIA_PREFIX,
            IBC_SUDO_ADDRESS_BYTES,
            SUDO_ADDRESS_BYTES,
        },
    };

    #[tokio::test]
    async fn should_fail_construction_if_address_not_base_prefixed() {
        // `IBC_SUDO` initialized as the IBC sudo address.
        let fixture = Fixture::default_initialized().await;
        let tx_signer = *IBC_SUDO_ADDRESS_BYTES;

        let prefix = "different_prefix";
        let address = address_with_prefix([50; ADDRESS_LEN], prefix);
        let action = IbcRelayerChange::Addition(address);
        let err = fixture
            .new_checked_action(action, tx_signer)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!("address has prefix `{prefix}` but only `{ASTRIA_PREFIX}` is permitted"),
        );

        let action = IbcRelayerChange::Removal(address);
        let err = fixture
            .new_checked_action(action, tx_signer)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!("address has prefix `{prefix}` but only `{ASTRIA_PREFIX}` is permitted"),
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_signer_is_not_ibc_sudo_address() {
        let fixture = Fixture::default_initialized().await;
        // Use a signer address different from the IBC sudo address.
        let tx_signer = [2; ADDRESS_LEN];
        assert_ne!(*IBC_SUDO_ADDRESS_BYTES, tx_signer);

        let address = astria_address(&[50; ADDRESS_LEN]);
        let addition_action = IbcRelayerChange::Addition(address);
        let removal_action = IbcRelayerChange::Removal(address);

        let err = fixture
            .new_checked_action(addition_action, tx_signer)
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change ibc relayer",
        );

        let err = fixture
            .new_checked_action(removal_action, tx_signer)
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change ibc relayer",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_is_not_ibc_sudo_address() {
        let mut fixture = Fixture::default_initialized().await;
        let tx_signer = *IBC_SUDO_ADDRESS_BYTES;

        let address = astria_address(&[50; ADDRESS_LEN]);
        let addition_action = IbcRelayerChange::Addition(address);
        let removal_action = IbcRelayerChange::Removal(address);

        // Construct checked IBC relayer change actions while the sudo address is still the
        // tx signer so construction succeeds.
        let checked_addition_action: CheckedIbcRelayerChange = fixture
            .new_checked_action(addition_action, tx_signer)
            .await
            .unwrap()
            .into();
        let checked_removal_action: CheckedIbcRelayerChange = fixture
            .new_checked_action(removal_action, tx_signer)
            .await
            .unwrap()
            .into();

        // Change the IBC sudo address to something other than the tx signer.
        let ibc_sudo_change = IbcSudoChange {
            new_address: astria_address(&[2; ADDRESS_LEN]),
        };
        let checked_ibc_sudo_change: CheckedIbcSudoChange = fixture
            .new_checked_action(ibc_sudo_change, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_ibc_sudo_change
            .execute(fixture.state_mut())
            .await
            .unwrap();
        let new_ibc_sudo_address = fixture.state().get_ibc_sudo_address().await.unwrap();
        assert_ne!(tx_signer, new_ibc_sudo_address);

        // Try to execute the checked actions now - should fail due to signer no longer being
        // authorized.
        let err = checked_addition_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change ibc relayer",
        );

        let err = checked_removal_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change ibc relayer",
        );
    }

    #[tokio::test]
    async fn should_execute() {
        let mut fixture = Fixture::default_initialized().await;
        let tx_signer = *IBC_SUDO_ADDRESS_BYTES;

        let address = astria_address(&[50; ADDRESS_LEN]);
        assert!(!fixture.state().is_ibc_relayer(&address).await.unwrap());

        let addition_action = IbcRelayerChange::Addition(address);
        let checked_addition_action: CheckedIbcRelayerChange = fixture
            .new_checked_action(addition_action, tx_signer)
            .await
            .unwrap()
            .into();
        checked_addition_action
            .execute(fixture.state_mut())
            .await
            .unwrap();
        assert!(fixture.state().is_ibc_relayer(&address).await.unwrap());

        let removal_action = IbcRelayerChange::Removal(address);
        let checked_removal_action: CheckedIbcRelayerChange = fixture
            .new_checked_action(removal_action, tx_signer)
            .await
            .unwrap()
            .into();
        checked_removal_action
            .execute(fixture.state_mut())
            .await
            .unwrap();
        assert!(!fixture.state().is_ibc_relayer(&address).await.unwrap());
    }
}
