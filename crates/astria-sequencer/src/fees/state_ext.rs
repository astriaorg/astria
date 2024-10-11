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
        SequenceFeeComponents,
        SudoAddressChangeFeeComponents,
        TransferFeeComponents,
        ValidatorUpdateFeeComponents,
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
use tracing::instrument;

use super::{
    storage::{
        self,
        keys,
    },
    Fee,
    FeeHandler,
};
use crate::storage::StoredValue;

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    fn get_block_fees(&self) -> Result<Vec<Fee>> {
        let mut block_fees = self.object_get(keys::BLOCK);
        match block_fees {
            Some(_) => {}
            None => {
                block_fees = Some(vec![]);
            }
        }
        Ok(block_fees.expect("block fees should not be `None` after populating"))
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
    async fn get_sequence_fees(&self) -> Result<SequenceFeeComponents> {
        let bytes = self
            .get_raw(keys::SEQUENCE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw sequence fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("sequence fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::SequenceFeeComponentsStorage::try_from(value)
                    .map(SequenceFeeComponents::from)
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
}

impl<T: ?Sized + StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    /// Constructs and adds `Fee` object to the block fees vec.
    #[instrument(skip_all)]
    fn add_fee_to_block_fees<'a, TAsset, T: FeeHandler + Protobuf>(
        &mut self,
        _act: &T,
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
    fn put_sequence_fees(&mut self, fees: SequenceFeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::SequenceFeeComponentsStorage::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::SEQUENCE.to_string(), bytes);
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
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use astria_core::protocol::transaction::v1alpha1::action::Transfer;
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        app::test_utils::{
            get_alice_signing_key,
            initialize_app_with_storage,
        },
        test_utils::ASTRIA_PREFIX,
    };

    fn asset_0() -> asset::Denom {
        "asset_0".parse().unwrap()
    }

    fn asset_1() -> asset::Denom {
        "asset_1".parse().unwrap()
    }

    #[tokio::test]
    async fn block_fee_read_and_increase() {
        let (_, storage) = initialize_app_with_storage(None, vec![]).await;
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        let fee_balances_orig = state.get_block_fees().unwrap();
        assert!(fee_balances_orig.is_empty());

        // can write
        let asset = asset_0();
        let amount = 100u128;
        let transfer_action = Transfer {
            to: get_alice_signing_key().try_address(ASTRIA_PREFIX).unwrap(),
            amount: 100,
            asset: asset.clone(),
            fee_asset: asset.clone(),
        };
        state
            .add_fee_to_block_fees(
                &transfer_action,
                &asset,
                amount,
                TransactionId::new([0; 32]),
                0,
            )
            .unwrap();

        // holds expected
        let fee_balances_updated = state.get_block_fees().unwrap();
        assert_eq!(
            fee_balances_updated[0],
            Fee {
                action_name: "astria.protocol.transactions.v1alpha1.Transfer".to_string(),
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

        let transfer_action_first = Transfer {
            to: get_alice_signing_key().try_address(ASTRIA_PREFIX).unwrap(),
            amount: amount_first,
            asset: asset_first.clone(),
            fee_asset: asset_first.clone(),
        };
        let transfer_action_second = Transfer {
            to: get_alice_signing_key().try_address(ASTRIA_PREFIX).unwrap(),
            amount: amount_second,
            asset: asset_second.clone(),
            fee_asset: asset_second.clone(),
        };

        state
            .add_fee_to_block_fees(
                &transfer_action_first,
                &asset_first,
                amount_first,
                TransactionId::new([0; 32]),
                0,
            )
            .unwrap();
        state
            .add_fee_to_block_fees(
                &transfer_action_second,
                &asset_second,
                amount_second,
                TransactionId::new([0; 32]),
                1,
            )
            .unwrap();
        // holds expected
        let fee_balances = HashSet::<_>::from_iter(state.get_block_fees().unwrap());
        assert_eq!(
            fee_balances,
            HashSet::from_iter(vec![
                Fee {
                    action_name: "astria.protocol.transactions.v1alpha1.Transfer".to_string(),
                    asset: asset_first.to_ibc_prefixed().into(),
                    amount: amount_first,
                    source_transaction_id: TransactionId::new([0; 32]),
                    source_action_index: 0
                },
                Fee {
                    action_name: "astria.protocol.transactions.v1alpha1.Transfer".to_string(),
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
    async fn sequence_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = SequenceFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state.put_sequence_fees(fee_components).unwrap();
        let retrieved_fee = state.get_sequence_fees().await.unwrap();
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
}
