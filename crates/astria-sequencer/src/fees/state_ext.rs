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
    protocol::fees::v1::FeeComponents,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        Report,
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
use tendermint::abci::Event;
use tracing::{
    instrument,
    Level,
};

use super::{
    storage::keys::{
        self,
        extract_asset_from_allowed_asset_key,
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

    #[instrument(skip_all, err(level = Level::WARN))]
    async fn get_fees<'a, F>(&self) -> Result<Option<FeeComponents<F>>>
    where
        F: FeeHandler + ?Sized,
        FeeComponents<F>: TryFrom<StoredValue<'a>, Error = Report>,
    {
        let bytes = self
            .get_raw(&keys::name::<F>())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err_with(|| {
                format!(
                    "failed reading raw {} fee components from state",
                    F::snake_case_name()
                )
            })?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| FeeComponents::<F>::try_from(value).map(Some))
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all, err(level = Level::WARN))]
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
    // TODO(https://github.com/astriaorg/astria/issues/1845): This doesn't need to return a result
    /// Constructs and adds `Fee` object to the block fees vec.
    #[instrument(skip_all)]
    fn add_fee_to_block_fees<'a, TAsset, F: FeeHandler + ?Sized>(
        &mut self,
        asset: &'a TAsset,
        amount: u128,
        position_in_transaction: u64,
    ) where
        TAsset: Sync + std::fmt::Display,
        asset::IbcPrefixed: From<&'a TAsset>,
    {
        let current_fees: Option<Vec<Fee>> = self.object_get(keys::BLOCK);

        let fee = Fee {
            action_name: F::full_name(),
            asset: asset::IbcPrefixed::from(asset).into(),
            amount,
            position_in_transaction,
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
    }

    #[instrument(skip_all, err(level = Level::WARN))]
    fn put_fees<'a, F>(&mut self, fees: FeeComponents<F>) -> Result<()>
    where
        F: FeeHandler,
        StoredValue<'a>: From<FeeComponents<F>>,
    {
        let bytes = StoredValue::from(fees)
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(keys::name::<F>(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn delete_allowed_fee_asset<'a, TAsset>(&mut self, asset: &'a TAsset)
    where
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        self.delete(keys::allowed_asset(asset));
    }

    #[instrument(skip_all, err(level = Level::WARN))]
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
            ("actionName", fee.action_name.to_string()),
            ("asset", fee.asset.to_string()),
            ("feeAmount", fee.amount.to_string()),
            (
                "positionInTransaction",
                fee.position_in_transaction.to_string(),
            ),
        ],
    )
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        fmt::Debug,
    };

    use astria_core::protocol::transaction::v1::action::*;
    use cnidarium::StateDelta;
    use futures::{
        StreamExt as _,
        TryStreamExt as _,
    };
    use penumbra_ibc::IbcRelay;
    use tokio::pin;

    use super::*;
    use crate::app::benchmark_and_test_utils::initialize_app_with_storage;

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
        state.add_fee_to_block_fees::<_, Transfer>(&asset, amount, 0);

        // holds expected
        let fee_balances_updated = state.get_block_fees();
        assert_eq!(
            fee_balances_updated[0],
            Fee {
                action_name: "astria.protocol.transaction.v1.Transfer".to_string(),
                asset: asset.to_ibc_prefixed().into(),
                amount,
                position_in_transaction: 0
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

        state.add_fee_to_block_fees::<_, Transfer>(&asset_first, amount_first, 0);
        state.add_fee_to_block_fees::<_, Transfer>(&asset_second, amount_second, 1);
        // holds expected
        let fee_balances = HashSet::<_>::from_iter(state.get_block_fees());
        assert_eq!(
            fee_balances,
            HashSet::from_iter(vec![
                Fee {
                    action_name: "astria.protocol.transaction.v1.Transfer".to_string(),
                    asset: asset_first.to_ibc_prefixed().into(),
                    amount: amount_first,
                    position_in_transaction: 0
                },
                Fee {
                    action_name: "astria.protocol.transaction.v1.Transfer".to_string(),
                    asset: asset_second.to_ibc_prefixed().into(),
                    amount: amount_second,
                    position_in_transaction: 1
                },
            ]),
            "returned fee balance vector not what was expected"
        );
    }

    async fn fees_round_trip<'a, F>()
    where
        F: FeeHandler,
        FeeComponents<F>: TryFrom<StoredValue<'a>, Error = Report> + Debug,
        StoredValue<'a>: From<FeeComponents<F>>,
    {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = FeeComponents::<F>::new(123, 1);
        state.put_fees(fee_components).unwrap();
        let retrieved_fees = state.get_fees().await.unwrap();
        assert_eq!(retrieved_fees, Some(fee_components));
    }

    #[tokio::test]
    async fn transfer_fees_round_trip() {
        fees_round_trip::<Transfer>().await;
    }

    #[tokio::test]
    async fn rollup_data_submission_fees_round_trip() {
        fees_round_trip::<RollupDataSubmission>().await;
    }

    #[tokio::test]
    async fn ics20_withdrawal_fees_round_trip() {
        fees_round_trip::<Ics20Withdrawal>().await;
    }

    #[tokio::test]
    async fn init_bridge_account_fees_round_trip() {
        fees_round_trip::<InitBridgeAccount>().await;
    }

    #[tokio::test]
    async fn bridge_lock_fees_round_trip() {
        fees_round_trip::<BridgeLock>().await;
    }

    #[tokio::test]
    async fn bridge_unlock_fees_round_trip() {
        fees_round_trip::<BridgeUnlock>().await;
    }

    #[tokio::test]
    async fn bridge_sudo_change_fees_round_trip() {
        fees_round_trip::<BridgeSudoChange>().await;
    }

    #[tokio::test]
    async fn ibc_relay_fees_round_trip() {
        fees_round_trip::<IbcRelay>().await;
    }

    #[tokio::test]
    async fn validator_update_fees_round_trip() {
        fees_round_trip::<ValidatorUpdate>().await;
    }

    #[tokio::test]
    async fn fee_asset_change_fees_round_trip() {
        fees_round_trip::<FeeAssetChange>().await;
    }

    #[tokio::test]
    async fn fee_change_fees_round_trip() {
        fees_round_trip::<FeeChange>().await;
    }

    #[tokio::test]
    async fn ibc_relayer_change_fees_round_trip() {
        fees_round_trip::<IbcRelayerChange>().await;
    }

    #[tokio::test]
    async fn sudo_address_change_fees_round_trip() {
        fees_round_trip::<SudoAddressChange>().await;
    }

    #[tokio::test]
    async fn ibc_sudo_change_fees_round_trip() {
        fees_round_trip::<IbcSudoChange>().await;
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
        pin!(
            let assets = state.allowed_fee_assets();
        );
        assert_eq!(
            assets.next().await.transpose().unwrap(),
            Some(asset.to_ibc_prefixed()),
            "expected returned allowed fee assets to match what was written in"
        );

        // can delete
        state.delete_allowed_fee_asset(&asset);

        // see is deleted
        pin!(
            let assets = state.allowed_fee_assets();
        );
        assert_eq!(
            assets.next().await.transpose().unwrap(),
            None,
            "fee assets should be empty post delete"
        );
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
        let assets = state
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
