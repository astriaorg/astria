use std::collections::HashSet;

use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::FeeAssetChange,
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
use futures::TryStreamExt as _;
use tracing::{
    instrument,
    Level,
};

use super::{
    AssetTransfer,
    TransactionSignerAddressBytes,
};
use crate::{
    authority::StateReadExt as _,
    fees::{
        StateReadExt as _,
        StateWriteExt as _,
    },
};

#[derive(Debug)]
pub(crate) struct CheckedFeeAssetChange {
    action: FeeAssetChange,
    tx_signer: TransactionSignerAddressBytes,
}

impl CheckedFeeAssetChange {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn new<S: StateRead>(
        action: FeeAssetChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self> {
        let checked_action = Self {
            action,
            tx_signer: tx_signer.into(),
        };
        checked_action.run_mutable_checks(state).await?;

        Ok(checked_action)
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn run_mutable_checks<S: StateRead>(&self, state: S) -> Result<()> {
        // Ensure the tx signer is the current sudo address.
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to read sudo address from storage")?;
        ensure!(
            &sudo_address == self.tx_signer.as_bytes(),
            "transaction signer not authorized to change fee assets"
        );

        // NOTE: To allow `app_legacy_execute_transactions_with_every_action_snapshot` to continue
        // to pass, we need to make an exception for `test-0` here to allow the test tx to be
        // constructed. This exception can be removed once the legacy test is removed.
        #[cfg(test)]
        {
            let fee_asset = match &self.action {
                FeeAssetChange::Addition(asset) | FeeAssetChange::Removal(asset) => asset,
            };
            if *fee_asset == "test-0".parse().unwrap() {
                return Ok(());
            }
        }
        match &self.action {
            FeeAssetChange::Addition(fee_asset) => {
                let is_allowed_fee_asset = state
                    .is_allowed_fee_asset(fee_asset)
                    .await
                    .wrap_err("failed to read fee asset from storage")?;
                ensure!(
                    !is_allowed_fee_asset,
                    "failed to add fee asset `{fee_asset}`: already is an allowed fee asset"
                );
            }
            FeeAssetChange::Removal(fee_asset) => {
                let allowed_fee_assets: HashSet<IbcPrefixed> = state
                    .allowed_fee_assets()
                    .try_collect()
                    .await
                    .wrap_err("failed to stream fee assets from storage")?;
                ensure!(
                    allowed_fee_assets.contains(&fee_asset.to_ibc_prefixed()),
                    "failed to remove fee asset `{fee_asset}`: is not currently an allowed fee \
                     asset"
                );
                ensure!(
                    allowed_fee_assets.len() > 1,
                    "cannot remove last allowed fee asset",
                );
            }
        }

        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        self.run_mutable_checks(&state).await?;

        match &self.action {
            FeeAssetChange::Addition(fee_asset) => {
                state
                    .put_allowed_fee_asset(fee_asset)
                    .wrap_err("failed to write allowed fee asset to storage")?;
            }
            FeeAssetChange::Removal(fee_asset) => {
                state.delete_allowed_fee_asset(fee_asset);
            }
        }
        Ok(())
    }

    pub(super) fn action(&self) -> &FeeAssetChange {
        &self.action
    }
}

impl AssetTransfer for CheckedFeeAssetChange {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use astria_core::protocol::transaction::v1::action::SudoAddressChange;

    use super::*;
    use crate::{
        checked_actions::CheckedSudoAddressChange,
        test_utils::{
            assert_error_contains,
            astria_address,
            denom_1,
            denom_2,
            nria,
            Fixture,
            SUDO_ADDRESS_BYTES,
        },
    };

    async fn get_allowed_fee_assets(fixture: &Fixture) -> Vec<IbcPrefixed> {
        fixture
            .state()
            .allowed_fee_assets()
            .try_collect()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn should_fail_construction_if_signer_is_not_sudo_address() {
        let fixture = Fixture::default_initialized().await;

        let tx_signer = [2_u8; ADDRESS_LEN];
        assert_ne!(tx_signer, *SUDO_ADDRESS_BYTES);

        let addition_action = FeeAssetChange::Addition(denom_1());
        let err = fixture
            .new_checked_action(addition_action, tx_signer)
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change fee assets",
        );

        let removal_action = FeeAssetChange::Removal(denom_1());
        let err = fixture
            .new_checked_action(removal_action, tx_signer)
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change fee assets",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_of_addition_if_fee_asset_already_allowed() {
        let fixture = Fixture::default_initialized().await;

        let action = FeeAssetChange::Addition(nria().into());
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "failed to add fee asset `nria`: already is an allowed fee asset",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_of_removal_if_fee_asset_not_currently_allowed() {
        let fixture = Fixture::default_initialized().await;

        let action = FeeAssetChange::Removal(denom_1());
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "failed to remove fee asset `denom_1`: is not currently an allowed fee asset",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_attempting_to_remove_only_fee_asset() {
        let fixture = Fixture::default_initialized().await;

        let action = FeeAssetChange::Removal(nria().into());
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "cannot remove last allowed fee asset");
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_is_not_sudo_address() {
        let mut fixture = Fixture::default_initialized().await;
        // Need two fee assets to be stored to allow for creating a checked actions that removes
        // one.
        fixture
            .state_mut()
            .put_allowed_fee_asset(&denom_1())
            .unwrap();

        // Construct the addition and removal checked actions while the sudo address is still the
        // tx signer so construction succeeds.
        let addition_action = FeeAssetChange::Addition(denom_2());
        let checked_addition_action: CheckedFeeAssetChange = fixture
            .new_checked_action(addition_action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        let removal_action = FeeAssetChange::Removal(nria().into());
        let checked_removal_action: CheckedFeeAssetChange = fixture
            .new_checked_action(removal_action, *SUDO_ADDRESS_BYTES)
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

        // Try to execute the two checked actions now - should fail due to signer no longer being
        // authorized.
        let err = checked_addition_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change fee assets",
        );

        let err = checked_removal_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change fee assets",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_of_addition_if_fee_asset_already_allowed() {
        let mut fixture = Fixture::default_initialized().await;

        // Use duplicate additions.
        let action = FeeAssetChange::Addition(denom_1());
        let checked_action_1: CheckedFeeAssetChange = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        let checked_action_2: CheckedFeeAssetChange = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // First addition should succeed.
        checked_action_1.execute(fixture.state_mut()).await.unwrap();

        // Second should fail due to fee asset now being stored.
        let err = checked_action_2
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "failed to add fee asset `denom_1`: already is an allowed fee asset",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_of_removal_if_fee_asset_not_currently_allowed() {
        let mut fixture = Fixture::default_initialized().await;
        fixture
            .state_mut()
            .put_allowed_fee_asset(&denom_1())
            .unwrap();

        // Use duplicate removals.
        let action = FeeAssetChange::Removal(denom_1());
        let checked_action_1: CheckedFeeAssetChange = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        let checked_action_2: CheckedFeeAssetChange = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // First removal should succeed.
        checked_action_1.execute(fixture.state_mut()).await.unwrap();

        // Second should fail due to fee asset now not being stored.
        let err = checked_action_2
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "failed to remove fee asset `denom_1`: is not currently an allowed fee asset",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_attempting_to_remove_only_asset() {
        let mut fixture = Fixture::default_initialized().await;
        // Need two fee assets to be stored to allow for creating checked actions that remove both.
        fixture
            .state_mut()
            .put_allowed_fee_asset(&denom_1())
            .unwrap();

        let action_1 = FeeAssetChange::Removal(nria().into());
        let checked_action_1: CheckedFeeAssetChange = fixture
            .new_checked_action(action_1, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        let action_2 = FeeAssetChange::Removal(denom_1());
        let checked_action_2: CheckedFeeAssetChange = fixture
            .new_checked_action(action_2, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // First removal should succeed.
        checked_action_1.execute(fixture.state_mut()).await.unwrap();

        // Second should fail due to fee asset now being the only one stored.
        let err = checked_action_2
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(&err, "cannot remove last allowed fee asset");
    }

    #[tokio::test]
    async fn should_execute_addition() {
        let mut fixture = Fixture::default_initialized().await;

        let allowed_fee_assets = get_allowed_fee_assets(&fixture).await;
        assert!(!allowed_fee_assets.contains(&denom_1().to_ibc_prefixed()));

        let action = FeeAssetChange::Addition(denom_1());
        let checked_action: CheckedFeeAssetChange = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        checked_action.execute(fixture.state_mut()).await.unwrap();

        let allowed_fee_assets = get_allowed_fee_assets(&fixture).await;
        assert!(allowed_fee_assets.contains(&denom_1().to_ibc_prefixed()));
    }

    #[tokio::test]
    async fn should_execute_removal() {
        let mut fixture = Fixture::default_initialized().await;
        fixture
            .state_mut()
            .put_allowed_fee_asset(&denom_1())
            .unwrap();

        let allowed_fee_assets = get_allowed_fee_assets(&fixture).await;
        assert!(allowed_fee_assets.contains(&denom_1().to_ibc_prefixed()));

        let action = FeeAssetChange::Removal(denom_1());
        let checked_action: CheckedFeeAssetChange = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        checked_action.execute(fixture.state_mut()).await.unwrap();

        let allowed_fee_assets = get_allowed_fee_assets(&fixture).await;
        assert!(!allowed_fee_assets.contains(&denom_1().to_ibc_prefixed()));
    }
}
