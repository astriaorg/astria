use std::borrow::Cow;

use astria_core::{
    primitive::v1::{
        asset,
        TransactionId,
    },
    protocol::fees::v1alpha1::{
        BridgeLockFeeComponents,
        BridgeSudoChangeFeeComponents,
        BridgeUnlockFeeComponents,
        FeeAssetChangeFeeComponents,
        FeeChangeFeeComponents,
        IbcRelayFeeComponents,
        IbcRelayerChangeFeeComponents,
        IbcSudoChangeFeeComponents,
        Ics20WithdrawalFeeComponents,
        InitBridgeAccountFeeComponents,
        RollupDataSubmissionFeeComponents,
        SudoAddressChangeFeeComponents,
        TransferFeeComponents,
        ValidatorUpdateFeeComponents,
        ValidatorUpdateV2FeeComponents,
    },
    Protobuf,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        eyre,
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use futures::StreamExt as _;
use tracing::instrument;

use super::{
    storage::{
        self,
        keys::{
            self,
            extract_asset_from_allowed_asset_key,
        },
    },
    Fee,
    FeeHandler,
};
use crate::storage::StoredValue;

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    fn get_block_fees(&self) -> Vec<Fee> {
        self.object_get(keys::BLOCK).unwrap_or_default()
    }

    #[instrument(skip_all)]
    async fn get_transfer_fees(&self) -> Result<TransferFeeComponents> {
        let bytes = self
            .get_raw(keys::TRANSFER)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw transfer fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("transfer fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::TransferFeeComponentsStorage::try_from(value)
                    .map(TransferFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_rollup_data_submission_fees(&self) -> Result<RollupDataSubmissionFeeComponents> {
        let bytes = self
            .get_raw(keys::ROLLUP_DATA_SUBMISSION)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw sequence fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("sequence fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::RollupDataSubmissionFeeComponentsStorage::try_from(value)
                    .map(RollupDataSubmissionFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_ics20_withdrawal_fees(&self) -> Result<Ics20WithdrawalFeeComponents> {
        let bytes = self
            .get_raw(keys::ICS20_WITHDRAWAL)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw ics20 withdrawal fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("ics20 withdrawal fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::Ics20WithdrawalFeeComponentsStorage::try_from(value)
                    .map(Ics20WithdrawalFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_init_bridge_account_fees(&self) -> Result<InitBridgeAccountFeeComponents> {
        let bytes = self
            .get_raw(keys::INIT_BRIDGE_ACCOUNT)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw init bridge account fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("init bridge account fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::InitBridgeAccountFeeComponentsStorage::try_from(value)
                    .map(InitBridgeAccountFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_lock_fees(&self) -> Result<BridgeLockFeeComponents> {
        let bytes = self
            .get_raw(keys::BRIDGE_LOCK)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge lock fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("bridge lock fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::BridgeLockFeeComponentsStorage::try_from(value)
                    .map(BridgeLockFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_unlock_fees(&self) -> Result<BridgeUnlockFeeComponents> {
        let bytes = self
            .get_raw(keys::BRIDGE_UNLOCK)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge unlock fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("bridge unlock fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::BridgeUnlockFeeComponentsStorage::try_from(value)
                    .map(BridgeUnlockFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_sudo_change_fees(&self) -> Result<BridgeSudoChangeFeeComponents> {
        let bytes = self
            .get_raw(keys::BRIDGE_SUDO_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge sudo change fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("bridge sudo change fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::BridgeSudoChangeFeeComponentsStorage::try_from(value)
                    .map(BridgeSudoChangeFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_ibc_relay_fees(&self) -> Result<IbcRelayFeeComponents> {
        let bytes = self
            .get_raw(keys::IBC_RELAY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw ibc relay fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("ibc relay fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::IbcRelayFeeComponentsStorage::try_from(value)
                    .map(IbcRelayFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    /// **NOTE**: Deprecated. Use [`ValidatorUpdateV2`] action instead.
    #[instrument(skip_all)]
    async fn get_validator_update_fees(&self) -> Result<ValidatorUpdateFeeComponents> {
        let bytes = self
            .get_raw(keys::VALIDATOR_UPDATE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw validator update fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("validator update fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::ValidatorUpdateFeeComponentsStorage::try_from(value)
                    .map(ValidatorUpdateFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_fee_asset_change_fees(&self) -> Result<FeeAssetChangeFeeComponents> {
        let bytes = self
            .get_raw(keys::FEE_ASSET_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw fee asset change fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("fee asset change fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::FeeAssetChangeFeeComponentsStorage::try_from(value)
                    .map(FeeAssetChangeFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_fee_change_fees(&self) -> Result<FeeChangeFeeComponents> {
        let bytes = self
            .get_raw(keys::FEE_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw fee change fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("fee change fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::FeeChangeFeeComponentsStorage::try_from(value)
                    .map(FeeChangeFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_ibc_relayer_change_fees(&self) -> Result<IbcRelayerChangeFeeComponents> {
        let bytes = self
            .get_raw(keys::IBC_RELAYER_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw ibc relayer change fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("ibc relayer change fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::IbcRelayerChangeFeeComponentsStorage::try_from(value)
                    .map(IbcRelayerChangeFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_sudo_address_change_fees(&self) -> Result<SudoAddressChangeFeeComponents> {
        let bytes = self
            .get_raw(keys::SUDO_ADDRESS_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw sudo address change fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("sudo address change fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::SudoAddressChangeFeeComponentsStorage::try_from(value)
                    .map(SudoAddressChangeFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_ibc_sudo_change_fees(&self) -> Result<IbcSudoChangeFeeComponents> {
        let bytes = self
            .get_raw(keys::IBC_SUDO_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw ibc sudo change fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("ibc sudo change fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::IbcSudoChangeFeeComponentsStorage::try_from(value)
                    .map(IbcSudoChangeFeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_validator_update_v2_fees(&self) -> Result<ValidatorUpdateV2FeeComponents> {
        let bytes = self
            .get_raw(keys::VALIDATOR_UPDATE_V2)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw validator update (v2) fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("validator update (v2) fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::ValidatorUpdateV2FeeComponentsStorage::try_from(value)
                    .map(ValidatorUpdateV2FeeComponents::from)
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn is_allowed_fee_asset<'a, TAsset>(&self, asset: &'a TAsset) -> Result<bool>
    where
        TAsset: Sync,
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        Ok(self
            .get_raw(&keys::allowed_asset(asset))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read raw fee asset from state")?
            .is_some())
    }

    #[instrument(skip_all)]
    async fn get_allowed_fee_assets(&self) -> Result<Vec<asset::IbcPrefixed>> {
        let mut assets = Vec::new();

        let mut stream = std::pin::pin!(self.prefix_raw(keys::ALLOWED_ASSET_PREFIX));
        while let Some(Ok((key, _))) = stream.next().await {
            let asset =
                extract_asset_from_allowed_asset_key(&key).wrap_err("failed to extract asset")?;
            assets.push(asset);
        }

        Ok(assets)
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    /// Constructs and adds `Fee` object to the block fees vec.
    #[instrument(skip_all)]
    fn add_fee_to_block_fees<'a, TAsset, T: FeeHandler + Protobuf>(
        &mut self,
        asset: &'a TAsset,
        amount: u128,
        source_transaction_id: TransactionId,
        source_action_index: u64,
    ) -> Result<()>
    where
        TAsset: Sync + std::fmt::Display,
        asset::IbcPrefixed: From<&'a TAsset>,
    {
        let current_fees: Option<Vec<Fee>> = self.object_get(keys::BLOCK);

        let fee = Fee {
            action_name: T::full_name(),
            asset: asset::IbcPrefixed::from(asset).into(),
            amount,
            source_transaction_id,
            source_action_index,
        };
        let new_fees = if let Some(mut fees) = current_fees {
            fees.push(fee);
            fees
        } else {
            vec![fee]
        };

        self.object_put(keys::BLOCK, new_fees);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_transfer_fees(&mut self, fees: TransferFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::TransferFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::TRANSFER.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_rollup_data_submission_fees(
        &mut self,
        fees: RollupDataSubmissionFeeComponents,
    ) -> Result<()> {
        let bytes = StoredValue::from(storage::RollupDataSubmissionFeeComponentsStorage::from(
            fees,
        ))
        .serialize()
        .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::ROLLUP_DATA_SUBMISSION.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_ics20_withdrawal_fees(&mut self, fees: Ics20WithdrawalFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::Ics20WithdrawalFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::ICS20_WITHDRAWAL.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_init_bridge_account_fees(&mut self, fees: InitBridgeAccountFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::InitBridgeAccountFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::INIT_BRIDGE_ACCOUNT.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_lock_fees(&mut self, fees: BridgeLockFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::BridgeLockFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::BRIDGE_LOCK.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_unlock_fees(&mut self, fees: BridgeUnlockFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::BridgeUnlockFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::BRIDGE_UNLOCK.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_sudo_change_fees(&mut self, fees: BridgeSudoChangeFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::BridgeSudoChangeFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::BRIDGE_SUDO_CHANGE.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_ibc_relay_fees(&mut self, fees: IbcRelayFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::IbcRelayFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::IBC_RELAY.to_string(), bytes);
        Ok(())
    }

    /// **NOTE**: Deprecated. Use [`ValidatorUpdateV2`] action instead.
    #[instrument(skip_all)]
    fn put_validator_update_fees(&mut self, fees: ValidatorUpdateFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::ValidatorUpdateFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::VALIDATOR_UPDATE.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_fee_asset_change_fees(&mut self, fees: FeeAssetChangeFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::FeeAssetChangeFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::FEE_ASSET_CHANGE.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_fee_change_fees(&mut self, fees: FeeChangeFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::FeeChangeFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::FEE_CHANGE.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_ibc_relayer_change_fees(&mut self, fees: IbcRelayerChangeFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::IbcRelayerChangeFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::IBC_RELAYER_CHANGE.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_sudo_address_change_fees(&mut self, fees: SudoAddressChangeFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::SudoAddressChangeFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::SUDO_ADDRESS_CHANGE.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_ibc_sudo_change_fees(&mut self, fees: IbcSudoChangeFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::IbcSudoChangeFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::IBC_SUDO_CHANGE.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_validator_update_v2_fees(&mut self, fees: ValidatorUpdateV2FeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::ValidatorUpdateV2FeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::VALIDATOR_UPDATE_V2.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn delete_allowed_fee_asset<'a, TAsset>(&mut self, asset: &'a TAsset)
    where
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        self.delete(keys::allowed_asset(asset));
    }

    #[instrument(skip_all)]
    fn put_allowed_fee_asset<'a, TAsset>(&mut self, asset: &'a TAsset) -> Result<()>
    where
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        let bytes = StoredValue::Unit
            .serialize()
            .context("failed to serialize unit for allowed fee asset")?;
        self.put_raw(keys::allowed_asset(asset), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use astria_core::protocol::transaction::v1alpha1::action::Transfer;
    use cnidarium::StateDelta;

    use super::*;
    use crate::app::test_utils::initialize_app_with_storage;

    fn asset_0() -> asset::Denom {
        "asset_0".parse().unwrap()
    }

    fn asset_1() -> asset::Denom {
        "asset_1".parse().unwrap()
    }

    fn asset_2() -> asset::Denom {
        "asset_2".parse().unwrap()
    }

    #[tokio::test]
    async fn block_fee_read_and_increase() {
        let (_, storage) = initialize_app_with_storage(None, vec![]).await;
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        let fee_balances_orig = state.get_block_fees();
        assert!(fee_balances_orig.is_empty());

        // can write
        let asset = asset_0();
        let amount = 100u128;
        state
            .add_fee_to_block_fees::<_, Transfer>(&asset, amount, TransactionId::new([0; 32]), 0)
            .unwrap();

        // holds expected
        let fee_balances_updated = state.get_block_fees();
        assert_eq!(
            fee_balances_updated[0],
            Fee {
                action_name: "astria.protocol.transaction.v1alpha1.Transfer".to_string(),
                asset: asset.to_ibc_prefixed().into(),
                amount,
                source_transaction_id: TransactionId::new([0; 32]),
                source_action_index: 0
            },
            "fee balances are not what they were expected to be"
        );
    }

    #[tokio::test]
    async fn block_fee_read_and_increase_can_delete() {
        let (_, storage) = initialize_app_with_storage(None, vec![]).await;
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write
        let asset_first = asset_0();
        let asset_second = asset_1();
        let amount_first = 100u128;
        let amount_second = 200u128;

        state
            .add_fee_to_block_fees::<_, Transfer>(
                &asset_first,
                amount_first,
                TransactionId::new([0; 32]),
                0,
            )
            .unwrap();
        state
            .add_fee_to_block_fees::<_, Transfer>(
                &asset_second,
                amount_second,
                TransactionId::new([0; 32]),
                1,
            )
            .unwrap();
        // holds expected
        let fee_balances = HashSet::<_>::from_iter(state.get_block_fees());
        assert_eq!(
            fee_balances,
            HashSet::from_iter(vec![
                Fee {
                    action_name: "astria.protocol.transaction.v1alpha1.Transfer".to_string(),
                    asset: asset_first.to_ibc_prefixed().into(),
                    amount: amount_first,
                    source_transaction_id: TransactionId::new([0; 32]),
                    source_action_index: 0
                },
                Fee {
                    action_name: "astria.protocol.transaction.v1alpha1.Transfer".to_string(),
                    asset: asset_second.to_ibc_prefixed().into(),
                    amount: amount_second,
                    source_transaction_id: TransactionId::new([0; 32]),
                    source_action_index: 1
                },
            ]),
            "returned fee balance vector not what was expected"
        );
    }

    #[tokio::test]
    async fn transfer_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = TransferFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_transfer_fees(fee_components).unwrap();
        let retrieved_fee = state.get_transfer_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn rollup_data_submission_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = RollupDataSubmissionFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state
            .put_rollup_data_submission_fees(fee_components)
            .unwrap();
        let retrieved_fee = state.get_rollup_data_submission_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn ics20_withdrawal_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = Ics20WithdrawalFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_ics20_withdrawal_fees(fee_components).unwrap();
        let retrieved_fee = state.get_ics20_withdrawal_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn init_bridge_account_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = InitBridgeAccountFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_init_bridge_account_fees(fee_components).unwrap();
        let retrieved_fee = state.get_init_bridge_account_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn bridge_lock_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = BridgeLockFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_bridge_lock_fees(fee_components).unwrap();
        let retrieved_fee = state.get_bridge_lock_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn bridge_unlock_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = BridgeUnlockFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_bridge_unlock_fees(fee_components).unwrap();
        let retrieved_fee = state.get_bridge_unlock_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn bridge_sudo_change_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = BridgeSudoChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_bridge_sudo_change_fees(fee_components).unwrap();
        let retrieved_fee = state.get_bridge_sudo_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn ibc_relay_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = IbcRelayFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_ibc_relay_fees(fee_components).unwrap();
        let retrieved_fee = state.get_ibc_relay_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn validator_update_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = ValidatorUpdateFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_validator_update_fees(fee_components).unwrap();
        let retrieved_fee = state.get_validator_update_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn fee_asset_change_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = FeeAssetChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_fee_asset_change_fees(fee_components).unwrap();
        let retrieved_fee = state.get_fee_asset_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn fee_change_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = FeeChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_fee_change_fees(fee_components).unwrap();
        let retrieved_fee = state.get_fee_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn ibc_relayer_change_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = IbcRelayerChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_ibc_relayer_change_fees(fee_components).unwrap();
        let retrieved_fee = state.get_ibc_relayer_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn sudo_address_change_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = SudoAddressChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_sudo_address_change_fees(fee_components).unwrap();
        let retrieved_fee = state.get_sudo_address_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn ibc_sudo_change_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = IbcSudoChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_ibc_sudo_change_fees(fee_components).unwrap();
        let retrieved_fee = state.get_ibc_sudo_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn validator_update_v2_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = ValidatorUpdateV2FeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_validator_update_v2_fees(fee_components).unwrap();
        let retrieved_fee = state.get_validator_update_v2_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn is_allowed_fee_asset() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // non-existent fees assets return false
        let asset = asset_0();
        assert!(
            !state
                .is_allowed_fee_asset(&asset)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to return false"
        );

        // existent fee assets return true
        state.put_allowed_fee_asset(&asset).unwrap();
        assert!(
            state
                .is_allowed_fee_asset(&asset)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
    }

    #[tokio::test]
    async fn can_delete_allowed_fee_assets_simple() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // setup fee asset
        let asset = asset_0();
        state.put_allowed_fee_asset(&asset).unwrap();
        assert!(
            state
                .is_allowed_fee_asset(&asset)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );

        // see can get fee asset
        let assets = state.get_allowed_fee_assets().await.unwrap();
        assert_eq!(
            assets,
            vec![asset.to_ibc_prefixed()],
            "expected returned allowed fee assets to match what was written in"
        );

        // can delete
        state.delete_allowed_fee_asset(&asset);

        // see is deleted
        let assets = state.get_allowed_fee_assets().await.unwrap();
        assert!(assets.is_empty(), "fee assets should be empty post delete");
    }

    #[tokio::test]
    async fn can_delete_allowed_fee_assets_complex() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // setup fee assets
        let asset_first = asset_0();
        state.put_allowed_fee_asset(&asset_first).unwrap();
        assert!(
            state
                .is_allowed_fee_asset(&asset_first)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
        let asset_second = asset_1();
        state.put_allowed_fee_asset(&asset_second).unwrap();
        assert!(
            state
                .is_allowed_fee_asset(&asset_second)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
        let asset_third = asset_2();
        state.put_allowed_fee_asset(&asset_third).unwrap();
        assert!(
            state
                .is_allowed_fee_asset(&asset_third)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );

        // can delete
        state.delete_allowed_fee_asset(&asset_second);

        // see is deleted
        let assets = HashSet::<_>::from_iter(state.get_allowed_fee_assets().await.unwrap());
        assert_eq!(
            assets,
            HashSet::from_iter(vec![
                asset_first.to_ibc_prefixed(),
                asset_third.to_ibc_prefixed()
            ]),
            "delete for allowed fee asset did not behave as expected"
        );
    }
}
