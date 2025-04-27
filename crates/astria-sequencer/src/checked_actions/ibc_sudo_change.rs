use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::IbcSudoChange,
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
    authority::StateReadExt as _,
    ibc::StateWriteExt as _,
};

#[derive(Debug)]
pub(crate) struct CheckedIbcSudoChange {
    action: IbcSudoChange,
    tx_signer: TransactionSignerAddressBytes,
}

impl CheckedIbcSudoChange {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn new<S: StateRead>(
        action: IbcSudoChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self> {
        // Run immutable checks for base prefix.
        state
            .ensure_base_prefix(&action.new_address)
            .await
            .wrap_err("new ibc sudo address has an unsupported prefix")?;

        let checked_action = Self {
            action,
            tx_signer: tx_signer.into(),
        };
        checked_action.run_mutable_checks(state).await?;

        Ok(checked_action)
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn run_mutable_checks<S: StateRead>(&self, state: S) -> Result<()> {
        // Check that the signer of this tx is the authorized sudo address.
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to read sudo address from storage")?;
        ensure!(
            &sudo_address == self.tx_signer.as_bytes(),
            "transaction signer not authorized to change ibc sudo address",
        );

        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        self.run_mutable_checks(&state).await?;
        state
            .put_ibc_sudo_address(self.action.new_address)
            .wrap_err("failed to write ibc sudo address to storage")
    }

    pub(super) fn action(&self) -> &IbcSudoChange {
        &self.action
    }
}

impl AssetTransfer for CheckedIbcSudoChange {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::Address,
        protocol::transaction::v1::action::SudoAddressChange,
    };

    use super::*;
    use crate::{
        checked_actions::CheckedSudoAddressChange,
        ibc::StateReadExt as _,
        test_utils::{
            assert_error_contains,
            astria_address,
            Fixture,
            ASTRIA_PREFIX,
            SUDO_ADDRESS_BYTES,
        },
    };

    #[tokio::test]
    async fn should_fail_construction_if_new_ibc_sudo_address_not_base_prefixed() {
        let fixture = Fixture::default_initialized().await;

        let prefix = "different_prefix";
        let new_address = Address::builder()
            .array([50; ADDRESS_LEN])
            .prefix(prefix)
            .try_build()
            .unwrap();

        let action = IbcSudoChange {
            new_address,
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
    async fn should_fail_construction_if_signer_is_not_sudo_address() {
        let fixture = Fixture::default_initialized().await;

        let tx_signer = [2_u8; ADDRESS_LEN];
        assert_ne!(*SUDO_ADDRESS_BYTES, tx_signer);

        let action = IbcSudoChange {
            new_address: astria_address(&[3; ADDRESS_LEN]),
        };
        let err = fixture
            .new_checked_action(action, tx_signer)
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change ibc sudo address",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_is_not_sudo_address() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct a checked IBC sudo address change action while the sudo address is still the tx
        // signer so construction succeeds.
        let action = IbcSudoChange {
            new_address: astria_address(&[2; ADDRESS_LEN]),
        };
        let checked_action: CheckedIbcSudoChange = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        // Change the sudo address to something other than the tx signer.
        let sudo_address_change = SudoAddressChange {
            new_address: astria_address(&[2; ADDRESS_LEN]),
        };
        let checked_sudo_address_change: CheckedSudoAddressChange = fixture
            .new_checked_action(sudo_address_change, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_sudo_address_change
            .execute(fixture.state_mut())
            .await
            .unwrap();
        let new_sudo_address = fixture.state().get_sudo_address().await.unwrap();
        assert_ne!(*SUDO_ADDRESS_BYTES, new_sudo_address);

        // Try to execute the checked action now - should fail due to signer no longer being
        // authorized.
        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "transaction signer not authorized to change ibc sudo address",
        );
    }

    #[tokio::test]
    async fn should_execute() {
        let mut fixture = Fixture::default_initialized().await;
        let old_ibc_sudo_address = fixture.state().get_ibc_sudo_address().await.unwrap();

        let new_ibc_sudo_address = astria_address(&[2; ADDRESS_LEN]);
        assert_ne!(old_ibc_sudo_address, new_ibc_sudo_address.bytes());

        let action = IbcSudoChange {
            new_address: new_ibc_sudo_address,
        };
        let checked_action: CheckedIbcSudoChange = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        checked_action.execute(fixture.state_mut()).await.unwrap();
        let ibc_sudo_address = fixture.state().get_ibc_sudo_address().await.unwrap();
        assert_eq!(ibc_sudo_address, new_ibc_sudo_address.bytes());
    }
}
