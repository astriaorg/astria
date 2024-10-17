use std::{
    borrow::Cow,
    pin::Pin,
    task::{
        ready,
        Context,
        Poll,
    },
};

use astria_core::{
    primitive::v1::asset,
    protocol::fees::v1::{
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
    },
    Protobuf,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use futures::Stream;
use pin_project_lite::pin_project;
use tendermint::abci::{
    Event,
    EventAttributeIndexExt as _,
};
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
pin_project! {
    /// A stream of all allowed fee assets for a given state.
    pub(crate) struct AllowedFeeAssetsStream<S> {
        #[pin]
        underlying: S,
    }
}

impl<St> Stream for AllowedFeeAssetsStream<St>
where
    St: Stream<Item = astria_eyre::anyhow::Result<String>>,
{
    type Item = Result<asset::IbcPrefixed>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let key = match ready!(this.underlying.as_mut().poll_next(cx)) {
            Some(Ok(key)) => key,
            Some(Err(err)) => {
                return Poll::Ready(Some(Err(
                    anyhow_to_eyre(err).wrap_err("failed reading from state")
                )));
            }
            None => return Poll::Ready(None),
        };
        Poll::Ready(Some(
            extract_asset_from_allowed_asset_key(&key)
                .with_context(|| format!("failed to extract IBC prefixed asset from key `{key}`")),
        ))
    }
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    fn get_block_fees(&self) -> Vec<Fee> {
        self.object_get(keys::BLOCK).unwrap_or_default()
    }

    #[instrument(skip_all)]
    async fn get_transfer_fees(&self) -> Result<Option<TransferFeeComponents>> {
        let bytes = self
            .get_raw(keys::TRANSFER)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw transfer fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::TransferFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(TransferFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_rollup_data_submission_fees(
        &self,
    ) -> Result<Option<RollupDataSubmissionFeeComponents>> {
        let bytes = self
            .get_raw(keys::ROLLUP_DATA_SUBMISSION)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw sequence fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::RollupDataSubmissionFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(RollupDataSubmissionFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_ics20_withdrawal_fees(&self) -> Result<Option<Ics20WithdrawalFeeComponents>> {
        let bytes = self
            .get_raw(keys::ICS20_WITHDRAWAL)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw ics20 withdrawal fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::Ics20WithdrawalFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(Ics20WithdrawalFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_init_bridge_account_fees(&self) -> Result<Option<InitBridgeAccountFeeComponents>> {
        let bytes = self
            .get_raw(keys::INIT_BRIDGE_ACCOUNT)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw init bridge account fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::InitBridgeAccountFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(InitBridgeAccountFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_lock_fees(&self) -> Result<Option<BridgeLockFeeComponents>> {
        let bytes = self
            .get_raw(keys::BRIDGE_LOCK)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge lock fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::BridgeLockFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(BridgeLockFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_unlock_fees(&self) -> Result<Option<BridgeUnlockFeeComponents>> {
        let bytes = self
            .get_raw(keys::BRIDGE_UNLOCK)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge unlock fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::BridgeUnlockFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(BridgeUnlockFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_sudo_change_fees(&self) -> Result<Option<BridgeSudoChangeFeeComponents>> {
        let bytes = self
            .get_raw(keys::BRIDGE_SUDO_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge sudo change fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::BridgeSudoChangeFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(BridgeSudoChangeFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_ibc_relay_fees(&self) -> Result<Option<IbcRelayFeeComponents>> {
        let bytes = self
            .get_raw(keys::IBC_RELAY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw ibc relay fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::IbcRelayFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(IbcRelayFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_validator_update_fees(&self) -> Result<Option<ValidatorUpdateFeeComponents>> {
        let bytes = self
            .get_raw(keys::VALIDATOR_UPDATE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw validator update fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::ValidatorUpdateFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(ValidatorUpdateFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_fee_asset_change_fees(&self) -> Result<Option<FeeAssetChangeFeeComponents>> {
        let bytes = self
            .get_raw(keys::FEE_ASSET_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw fee asset change fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::FeeAssetChangeFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(FeeAssetChangeFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_fee_change_fees(&self) -> Result<Option<FeeChangeFeeComponents>> {
        let bytes = self
            .get_raw(keys::FEE_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw fee change fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::FeeChangeFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(FeeChangeFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_ibc_relayer_change_fees(&self) -> Result<Option<IbcRelayerChangeFeeComponents>> {
        let bytes = self
            .get_raw(keys::IBC_RELAYER_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw ibc relayer change fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::IbcRelayerChangeFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(IbcRelayerChangeFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_sudo_address_change_fees(&self) -> Result<Option<SudoAddressChangeFeeComponents>> {
        let bytes = self
            .get_raw(keys::SUDO_ADDRESS_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw sudo address change fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::SudoAddressChangeFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(SudoAddressChangeFeeComponents::from(fees)))
            })
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_ibc_sudo_change_fees(&self) -> Result<Option<IbcSudoChangeFeeComponents>> {
        let bytes = self
            .get_raw(keys::IBC_SUDO_CHANGE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw ibc sudo change fee components from state")?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::IbcSudoChangeFeeComponentsStorage::try_from(value)
                    .map(|fees| Some(IbcSudoChangeFeeComponents::from(fees)))
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
    fn allowed_fee_assets(&self) -> AllowedFeeAssetsStream<Self::PrefixKeysStream> {
        AllowedFeeAssetsStream {
            underlying: self.prefix_keys(keys::ALLOWED_ASSET_PREFIX),
        }
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
            source_action_index,
        };

        // Fee ABCI event recorded for reporting
        let fee_event = construct_tx_fee_event(&fee);
        self.record(fee_event);

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

/// Creates `abci::Event` of kind `tx.fees` for sequencer fee reporting
fn construct_tx_fee_event(fee: &Fee) -> Event {
    Event::new(
        "tx.fees",
        [
            ("actionName", fee.action_name.to_string()).index(),
            ("asset", fee.asset.to_string()).index(),
            ("feeAmount", fee.amount.to_string()).index(),
            ("positionInTransaction", fee.source_action_index.to_string()).index(),
        ],
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use astria_core::protocol::transaction::v1::action::Transfer;
    use futures::{
        StreamExt as _,
        TryStreamExt as _,
    };
    use tokio::pin;

    use super::*;
    use crate::{
        app::benchmark_and_test_utils::initialize_app_with_storage,
        storage::Storage,
    };

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
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // doesn't exist at first
        let fee_balances_orig = state_delta.get_block_fees();
        assert!(fee_balances_orig.is_empty());

        // can write
        let asset = asset_0();
        let amount = 100u128;
        state_delta
            .add_fee_to_block_fees::<_, Transfer>(&asset, amount, 0)
            .unwrap();

        // holds expected
        let fee_balances_updated = state_delta.get_block_fees();
        assert_eq!(
            fee_balances_updated[0],
            Fee {
                action_name: "astria.protocol.transaction.v1.Transfer".to_string(),
                asset: asset.to_ibc_prefixed().into(),
                amount,
                source_action_index: 0
            },
            "fee balances are not what they were expected to be"
        );
    }

    #[tokio::test]
    async fn block_fee_read_and_increase_can_delete() {
        let (_, storage) = initialize_app_with_storage(None, vec![]).await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // can write
        let asset_first = asset_0();
        let asset_second = asset_1();
        let amount_first = 100u128;
        let amount_second = 200u128;

        state_delta
            .add_fee_to_block_fees::<_, Transfer>(&asset_first, amount_first, 0)
            .unwrap();
        state_delta
            .add_fee_to_block_fees::<_, Transfer>(&asset_second, amount_second, 1)
            .unwrap();
        // holds expected
        let fee_balances = HashSet::<_>::from_iter(state_delta.get_block_fees());
        assert_eq!(
            fee_balances,
            HashSet::from_iter(vec![
                Fee {
                    action_name: "astria.protocol.transaction.v1.Transfer".to_string(),
                    asset: asset_first.to_ibc_prefixed().into(),
                    amount: amount_first,
                    source_action_index: 0
                },
                Fee {
                    action_name: "astria.protocol.transaction.v1.Transfer".to_string(),
                    asset: asset_second.to_ibc_prefixed().into(),
                    amount: amount_second,
                    source_action_index: 1
                },
            ]),
            "returned fee balance vector not what was expected"
        );
    }

    #[tokio::test]
    async fn transfer_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = TransferFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta.put_transfer_fees(fee_components).unwrap();
        let retrieved_fee = state_delta.get_transfer_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn rollup_data_submission_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = RollupDataSubmissionFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta
            .put_rollup_data_submission_fees(fee_components)
            .unwrap();
        let retrieved_fee = state_delta.get_rollup_data_submission_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn ics20_withdrawal_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = Ics20WithdrawalFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta
            .put_ics20_withdrawal_fees(fee_components)
            .unwrap();
        let retrieved_fee = state_delta.get_ics20_withdrawal_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn init_bridge_account_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = InitBridgeAccountFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta
            .put_init_bridge_account_fees(fee_components)
            .unwrap();
        let retrieved_fee = state_delta.get_init_bridge_account_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn bridge_lock_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = BridgeLockFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta.put_bridge_lock_fees(fee_components).unwrap();
        let retrieved_fee = state_delta.get_bridge_lock_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn bridge_unlock_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = BridgeUnlockFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta.put_bridge_unlock_fees(fee_components).unwrap();
        let retrieved_fee = state_delta.get_bridge_unlock_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn bridge_sudo_change_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = BridgeSudoChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta
            .put_bridge_sudo_change_fees(fee_components)
            .unwrap();
        let retrieved_fee = state_delta.get_bridge_sudo_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn ibc_relay_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = IbcRelayFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta.put_ibc_relay_fees(fee_components).unwrap();
        let retrieved_fee = state_delta.get_ibc_relay_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn validator_update_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = ValidatorUpdateFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta
            .put_validator_update_fees(fee_components)
            .unwrap();
        let retrieved_fee = state_delta.get_validator_update_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn fee_asset_change_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = FeeAssetChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta
            .put_fee_asset_change_fees(fee_components)
            .unwrap();
        let retrieved_fee = state_delta.get_fee_asset_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn fee_change_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = FeeChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta.put_fee_change_fees(fee_components).unwrap();
        let retrieved_fee = state_delta.get_fee_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn ibc_relayer_change_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = IbcRelayerChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta
            .put_ibc_relayer_change_fees(fee_components)
            .unwrap();
        let retrieved_fee = state_delta.get_ibc_relayer_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn sudo_address_change_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = SudoAddressChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta
            .put_sudo_address_change_fees(fee_components)
            .unwrap();
        let retrieved_fee = state_delta.get_sudo_address_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn ibc_sudo_change_fees_round_trip() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let fee_components = IbcSudoChangeFeeComponents {
            base: 123,
            multiplier: 1,
        };

        state_delta
            .put_ibc_sudo_change_fees(fee_components)
            .unwrap();
        let retrieved_fee = state_delta.get_ibc_sudo_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, Some(fee_components));
    }

    #[tokio::test]
    async fn is_allowed_fee_asset() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // non-existent fees assets return false
        let asset = asset_0();
        assert!(
            !state_delta
                .is_allowed_fee_asset(&asset)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to return false"
        );

        // existent fee assets return true
        state_delta.put_allowed_fee_asset(&asset).unwrap();
        assert!(
            state_delta
                .is_allowed_fee_asset(&asset)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
    }

    #[tokio::test]
    async fn can_delete_allowed_fee_assets_simple() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // setup fee asset
        let asset = asset_0();
        state_delta.put_allowed_fee_asset(&asset).unwrap();
        assert!(
            state_delta
                .is_allowed_fee_asset(&asset)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );

        // see can get fee asset
        pin!(
            let assets = state_delta.allowed_fee_assets();
        );
        assert_eq!(
            assets.next().await.transpose().unwrap(),
            Some(asset.to_ibc_prefixed()),
            "expected returned allowed fee assets to match what was written in"
        );

        // can delete
        state_delta.delete_allowed_fee_asset(&asset);

        // see is deleted
        pin!(
            let assets = state_delta.allowed_fee_assets();
        );
        assert_eq!(
            assets.next().await.transpose().unwrap(),
            None,
            "fee assets should be empty post delete"
        );
    }

    #[tokio::test]
    async fn can_delete_allowed_fee_assets_complex() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // setup fee assets
        let asset_first = asset_0();
        state_delta.put_allowed_fee_asset(&asset_first).unwrap();
        assert!(
            state_delta
                .is_allowed_fee_asset(&asset_first)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
        let asset_second = asset_1();
        state_delta.put_allowed_fee_asset(&asset_second).unwrap();
        assert!(
            state_delta
                .is_allowed_fee_asset(&asset_second)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );
        let asset_third = asset_2();
        state_delta.put_allowed_fee_asset(&asset_third).unwrap();
        assert!(
            state_delta
                .is_allowed_fee_asset(&asset_third)
                .await
                .expect("checking for allowed fee asset should not fail"),
            "fee asset was expected to be allowed"
        );

        // can delete
        state_delta.delete_allowed_fee_asset(&asset_second);

        // see is deleted
        let assets = state_delta
            .allowed_fee_assets()
            .try_collect::<HashSet<_>>()
            .await
            .unwrap();
        assert_eq!(
            assets,
            maplit::hashset!(asset_first.to_ibc_prefixed(), asset_third.to_ibc_prefixed()),
            "delete for allowed fee asset did not behave as expected"
        );
    }
}
