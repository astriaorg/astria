use astria_core::protocol::transaction::v1::action::FeeAssetChange;
use astria_eyre::eyre::{
    self,
    ensure,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;
use futures::StreamExt as _;
use tokio::pin;
use tracing::{
    instrument,
    Level,
};

use crate::{
    action_handler::ActionHandler,
    authority::StateReadExt as _,
    fees::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for FeeAssetChange {
    async fn check_stateless(&self) -> eyre::Result<()> {
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
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
    use astria_core::primitive::v1::TransactionId;
    use cnidarium::StateDelta;
    use futures::TryStreamExt as _;

    use super::*;
    use crate::{
        accounts::AddressBytes,
        action_handler::impls::test_utils::test_asset,
        authority::StateWriteExt,
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
            nria,
        },
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn fee_asset_change_addition_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let sudo_address = astria_address(&[1; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_sudo_address(sudo_address).unwrap();
        state.put_allowed_fee_asset(&nria()).unwrap();

        let fee_asset_change = FeeAssetChange::Addition(test_asset());
        fee_asset_change
            .check_and_execute(&mut state)
            .await
            .unwrap();

        let fee_assets = state
            .allowed_fee_assets()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert_eq!(fee_assets.len(), 2);
        assert!(fee_assets.contains(&nria().to_ibc_prefixed()));
        assert!(fee_assets.contains(&test_asset().to_ibc_prefixed()));
    }

    #[tokio::test]
    async fn fee_asset_change_removal_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let sudo_address = astria_address(&[1; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_sudo_address(sudo_address).unwrap();
        state.put_allowed_fee_asset(&nria()).unwrap();
        state.put_allowed_fee_asset(&test_asset()).unwrap();

        let fee_asset_change = FeeAssetChange::Removal(test_asset());
        fee_asset_change
            .check_and_execute(&mut state)
            .await
            .unwrap();

        let fee_assets = state
            .allowed_fee_assets()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert_eq!(fee_assets.len(), 1);
        assert!(fee_assets.contains(&nria().to_ibc_prefixed()));
    }

    #[tokio::test]
    async fn fee_asset_change_fails_if_signer_is_not_sudo_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let sudo_address = astria_address(&[1; 20]);
        let signer = astria_address(&[2; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: *signer.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_sudo_address(sudo_address).unwrap();

        let fee_asset_change = FeeAssetChange::Addition(test_asset());
        assert_eyre_error(
            &fee_asset_change
                .check_and_execute(&mut state)
                .await
                .unwrap_err(),
            "unauthorized address for fee asset change",
        );
    }

    #[tokio::test]
    async fn fee_asset_change_fails_if_attempting_to_remove_only_asset() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let sudo_address = astria_address(&[1; 20]);
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_sudo_address(sudo_address).unwrap();
        state.put_allowed_fee_asset(&nria()).unwrap();

        let fee_asset_change = FeeAssetChange::Removal(nria().into());
        assert_eyre_error(
            &fee_asset_change
                .check_and_execute(&mut state)
                .await
                .unwrap_err(),
            "cannot remove last allowed fee asset",
        );
    }
}
